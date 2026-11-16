//! `::file … interpolate=false`: a transcluded child's prose stays as
//! written while its frontmatter and directive inputs keep evaluating.
//!
//! Boundary: the library's public composition result (`compose_with` output,
//! report, and errors) over real files in a temp dir, F2 in-process, L1.

use super::*;
use crate::markdown::compose::policy::BlockingPolicy;
use crate::markdown::compose::shell_expansion::types::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionError, ShellExpansionOptions,
};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// The spec's literal-prose example plus an invalid call, a malformed
/// expression, and a literal token.
const EXAMPLES: &str = "# Examples\n\nUse `{{ name }}` to read a property.\nAn unknown name looks like {{ undeclared }}.\nWrite {{{ x }}} for an interpolation literal.\nA bad call: {{ length() }}.\nMalformed: {{ 1 + }}.\n";

fn write(dir: &Path, name: &str, text: &str) {
    std::fs::write(dir.join(name), text).unwrap();
}

/// Composes `root.md` in `dir` with `configure` applied to its options.
fn compose_root(
    dir: &Path,
    root_text: &str,
    configure: impl FnOnce(ComposeOptions) -> ComposeOptions,
) -> MarkdownResult<(Markdown, ComposeReport)> {
    let root = dir.join("root.md");
    std::fs::write(&root, root_text).unwrap();
    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = configure(ComposeOptions::new().with_source_file(root));
    md.compose_with(&crate::markdown::compose::test_request(options))
}

fn deny_all(options: ComposeOptions) -> ComposeOptions {
    options.with_blocking_policy(BlockingPolicy::default().deny_all())
}

fn messages(report: &ComposeReport) -> Vec<String> {
    report.warnings.iter().map(|warning| warning.message.clone()).collect()
}

/// Every line of `EXAMPLES` after its heading, as written.
fn example_lines() -> Vec<&'static str> {
    EXAMPLES.lines().skip(2).collect()
}

#[test]
fn literal_prose_survives_with_no_diagnostic_even_under_deny_all() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "child.md", EXAMPLES);
    let root = "---\nname: Caller\n---\nParent {{ name }}.\n\n::file ./child.md interpolate=false\n";

    let (composed, report) = compose_root(dir.path(), root, deny_all).unwrap();

    let text = composed.content();
    for line in example_lines() {
        assert!(text.contains(line), "{line:?} survives as written in:\n{text}");
    }
    assert!(text.contains("Parent Caller."), "the parent's own prose still interpolates:\n{text}");
    assert!(!text.contains("`Caller`"), "the child never reads the caller's name:\n{text}");
    assert!(report.warnings.is_empty(), "no warning: {:?}", messages(&report));
    assert_eq!(report.transclusions_applied, 1);
    assert_eq!(report.transclusions_skipped, 0);
}

#[test]
fn every_false_spelling_suppresses_and_true_spellings_interpolate() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "child.md", "Hello {{ name }}.\n");
    let root = |option: &str| format!("---\nname: World\n---\n::file ./child.md {option}\n");
    for option in ["interpolate=false", "interpolate=no", "interpolate=OFF", "interpolate=0", "interpolate=\"False\""] {
        let (composed, _) = compose_root(dir.path(), &root(option), |options| options).unwrap();
        assert!(composed.content().contains("Hello {{ name }}."), "{option}: {}", composed.content());
    }
    for option in ["", "interpolate=true", "interpolate=null", "interpolate=yes", "interpolate='1'"] {
        let (composed, _) = compose_root(dir.path(), &root(option), |options| options).unwrap();
        assert!(composed.content().contains("Hello World."), "{option:?}: {}", composed.content());
    }
}

/// The control for the suppression tests: without the option the same
/// examples are evaluated, so they warn, and `--deny all` stops on them.
#[test]
fn without_the_option_the_examples_are_evaluated() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "child.md", "Use `{{ name }}`.\nAn unknown name looks like {{ undeclared }}.\n");
    let root = "---\nname: Caller\n---\n::file ./child.md\n";

    let (composed, report) = compose_root(dir.path(), root, |options| options).unwrap();
    assert!(composed.content().contains("Use `Caller`."), "{}", composed.content());
    assert!(
        messages(&report).iter().any(|message| message.contains("undeclared")),
        "the unknown name warns: {:?}",
        messages(&report)
    );

    let blocked = compose_root(dir.path(), root, deny_all).unwrap_err();
    assert!(matches!(blocked, MarkdownError::BlockedDiagnostics { .. }), "{blocked:?}");

    write(dir.path(), "bad.md", "Malformed: {{ 1 + }}.\n");
    let (_, failed) = compose_root(dir.path(), "::file ./bad.md\n", |options| options).unwrap();
    assert_eq!(failed.transclusions_skipped, 1, "a malformed expression fails a normal inclusion");
    assert!(messages(&failed).iter().any(|message| message.contains("parse error")), "{:?}", messages(&failed));
}

#[test]
fn code_blocks_stay_literal_even_when_the_child_opts_into_code_interpolation() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "child.md",
        "---\ninterpolate_code_blocks: true\n---\nProse {{ name }}.\n\n```md\n{{ name }}\n```\n\n    {{ name }} indented\n",
    );
    let root = "---\nname: Caller\n---\n::file ./child.md interpolate=false\n";
    for configure in [
        (|options| options) as fn(ComposeOptions) -> ComposeOptions,
        |options: ComposeOptions| options.with_interpolate_code_blocks(true),
    ] {
        let (composed, report) = compose_root(dir.path(), root, configure).unwrap();
        let text = composed.content();
        assert!(!text.contains("Caller"), "nothing in the child interpolates:\n{text}");
        assert!(text.contains("```md\n{{ name }}\n```"), "{text}");
        assert!(text.contains("{{ name }} indented"), "{text}");
        assert!(report.warnings.is_empty(), "{:?}", messages(&report));
    }

    // The control: included normally, the child's opt-in interpolates code.
    let (composed, _) = compose_root(dir.path(), "---\nname: Caller\n---\n::file ./child.md\n", |options| options).unwrap();
    assert!(composed.content().contains("```md\nCaller\n```"), "{}", composed.content());
}

/// The child's frontmatter interpolates as usual and a `set` override still
/// outranks it; a condition reads the result while the prose stays literal.
#[test]
fn frontmatter_interpolates_and_set_overrides_keep_their_precedence() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "child.md",
        "---\nwho: Child\ngreeting: \"Hi {{ who }}\"\n---\n::block when=\"greeting == 'Hi Child'\"\nFrom frontmatter: {{ greeting }}\n::end-block\n::block when=\"who == 'Override'\"\nOverridden: {{ who }}\n::end-block\n",
    );

    let (plain, _) = compose_root(dir.path(), "::file ./child.md interpolate=false\n", |options| options).unwrap();
    assert!(plain.content().contains("From frontmatter: {{ greeting }}"), "{}", plain.content());
    assert!(!plain.content().contains("Overridden"), "{}", plain.content());

    let (overridden, _) =
        compose_root(dir.path(), "::file ./child.md interpolate=false set.who=\"Override\"\n", |options| options).unwrap();
    assert!(overridden.content().contains("Overridden: {{ who }}"), "{}", overridden.content());
    assert!(!overridden.content().contains("From frontmatter"), "the override reached the frontmatter template too");
}

#[test]
fn conditions_and_collections_execute_while_their_prose_stays_literal() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "child.md",
        "---\n$schema:\n  items: string[](required)\nshow: true\nhide: false\nitems: [a, b, c]\n---\n::block when=\"show\"\nShown: {{ name }}\n::end-block\n::block when=\"hide\"\nHidden: {{ name }}\n::end-block\n::map items \"item => item\" when=\"show\"\n- Item {{ item }}\n::end-map\n::filter items \"item => item != 'b'\"\n* Kept {{ item }}\n::end-filter\n",
    );

    let (composed, report) = compose_root(dir.path(), "::file ./child.md interpolate=false\n", deny_all).unwrap();

    let text = composed.content();
    assert!(text.contains("Shown: {{ name }}"), "{text}");
    assert!(!text.contains("Hidden"), "the false condition still drops its block:\n{text}");
    assert_eq!(text.matches("- Item {{ item }}").count(), 3, "the map repeats its literal body per item:\n{text}");
    assert_eq!(text.matches("* Kept {{ item }}").count(), 2, "the filter callback still runs:\n{text}");
    assert!(report.warnings.is_empty(), "{:?}", messages(&report));
}

/// A nested directive's target and option values evaluate; the nested child
/// inherits the disabled setting, so its prose stays literal while its
/// frontmatter sees the supplied value.
#[test]
fn nested_targets_and_option_values_evaluate() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "child.md",
        "---\nleaf: detail\n---\nChild {{ name }}.\n::file ./{{ leaf }}.md set.who=\"{{ name }}\"\n",
    );
    write(
        dir.path(),
        "detail.md",
        "---\nwho: nobody\n---\nDetail {{ who }}.\n::block when=\"who == 'Caller'\"\nSupplied.\n::end-block\n",
    );

    let (composed, report) =
        compose_root(dir.path(), "---\nname: Caller\n---\n::file ./child.md interpolate=false\n", |options| options).unwrap();

    let text = composed.content();
    assert!(text.contains("Child {{ name }}."), "{text}");
    assert!(text.contains("Detail {{ who }}."), "the nested child inherits the setting:\n{text}");
    assert!(text.contains("Supplied."), "the option value evaluated against the child's state:\n{text}");
    assert_eq!(report.transclusions_applied, 2);
}

struct RecordingApproval(Mutex<Vec<String>>);

impl ShellApprovalHandler for RecordingApproval {
    fn approve(&self, request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        self.0.lock().unwrap().push(request.raw_command);
        Ok(ShellApprovalDecision::AllowOnce)
    }
}

#[test]
fn shell_inputs_evaluate_and_run_with_the_interpolated_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "child.md",
        "---\nword: child-word\n---\nProse {{ word }}.\n\n::shell echo {{ word }}\n\n::shell-block\necho block-{{ word }}\n::end-block\n",
    );
    let approvals = Arc::new(RecordingApproval(Mutex::new(Vec::new())));
    let handler = Arc::clone(&approvals);

    let (composed, _) = compose_root(dir.path(), "::file ./child.md interpolate=false\n", |options| {
        options
            .with_config_snapshot(crate::markdown::compose::shell_expansion::test_shell_config(home.path(), &[]))
            .with_shell(ShellExpansionOptions { approval_handler: Some(handler), ..Default::default() })
    })
    .unwrap();

    let text = composed.content();
    assert!(text.contains("Prose {{ word }}."), "{text}");
    assert!(text.contains("child-word") && text.contains("block-child-word"), "both commands ran:\n{text}");
    let approved = approvals.0.lock().unwrap().clone();
    assert!(approved.iter().any(|command| command == "echo child-word"), "{approved:?}");
    assert!(approved.iter().any(|command| command.contains("echo block-child-word")), "{approved:?}");
    assert!(approved.iter().all(|command| !command.contains("{{")), "no command kept raw syntax: {approved:?}");
}

#[test]
fn replacement_exclusion_and_wrappers_still_transform_the_literal_prose() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "child.md",
        "# Kept\n\nSay {{ name }} with TOKEN.\n\n## Dropped\n\nGone {{ name }}.\n",
    );
    let root = "---\nname: Caller\n---\n::file ./child.md interpolate=false replace={\"TOKEN\":\"{{ name }}\"} exclude=\"## Dropped\" quotation=\"Source\"\n";

    let (composed, report) = compose_root(dir.path(), root, deny_all).unwrap();

    let text = composed.content();
    // The directive's `replace=` is the parent's input, so its value is
    // evaluated; the child's own prose stays literal around it.
    assert!(text.contains("Say {{ name }} with Caller."), "the replacement applied:\n{text}");
    assert!(!text.contains("Gone"), "the excluded section is gone:\n{text}");
    assert!(text.contains("> "), "the quotation wrapper applied:\n{text}");
    assert!(report.warnings.is_empty(), "{:?}", messages(&report));
}

/// One child included normally and with interpolation disabled, in both
/// orders and with the composed-child cache on and off: each inclusion keeps
/// its own result and warnings, and the parent's prose interpolates.
#[test]
fn the_same_child_composes_independently_under_each_setting() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "child.md", "Hello {{ name }}.\n");
    let orders = [
        "---\nname: World\n---\nTop {{ name }}.\n\n::file ./child.md\n\n::file ./child.md interpolate=false\n\nEnd {{ name }}.\n",
        "---\nname: World\n---\nTop {{ name }}.\n\n::file ./child.md interpolate=false\n\n::file ./child.md\n\nEnd {{ name }}.\n",
    ];
    for (index, root) in orders.iter().enumerate() {
        for mode in [CacheAccessMode::ReadWrite, CacheAccessMode::Off] {
            let (composed, report) = compose_root(dir.path(), root, |options| options.with_cache_access_mode(mode)).unwrap();
            let text = composed.content();
            let normal = text.find("Hello World.").unwrap_or_else(|| panic!("order {index} {mode:?}:\n{text}"));
            let literal = text.find("Hello {{ name }}.").unwrap_or_else(|| panic!("order {index} {mode:?}:\n{text}"));
            assert_eq!(normal < literal, index == 0, "order {index} {mode:?} keeps directive order:\n{text}");
            assert!(text.contains("Top World.") && text.contains("End World."), "{text}");
            assert!(report.warnings.is_empty(), "{:?}", messages(&report));
            assert_eq!(report.transclusions_applied, 2);
        }
    }
}

/// Under a disabled ancestor, a nested omitted, false, true, or null option
/// all leave the descendant literal, and an explicit true adds no warning.
#[test]
fn a_disabled_ancestor_cannot_be_re_enabled() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "leaf.md", "Leaf {{ name }}.\n");
    for option in ["", "interpolate=false", "interpolate=true", "interpolate=null"] {
        write(dir.path(), "middle.md", &format!("Middle {{{{ name }}}}.\n::file ./leaf.md {option}\n"));
        let (composed, report) =
            compose_root(dir.path(), "---\nname: Caller\n---\n::file ./middle.md interpolate=false\n", deny_all).unwrap();
        let text = composed.content();
        assert!(text.contains("Middle {{ name }}.") && text.contains("Leaf {{ name }}."), "{option:?}:\n{text}");
        assert!(report.warnings.is_empty(), "{option:?}: {:?}", messages(&report));
    }

    // The control: the setting never reaches a sibling of the disabled branch.
    write(dir.path(), "middle.md", "Middle {{ name }}.\n::file ./leaf.md\n");
    let (composed, _) = compose_root(
        dir.path(),
        "---\nname: Caller\n---\n::file ./middle.md interpolate=false\n\n::file ./leaf.md\n",
        |options| options,
    )
    .unwrap();
    assert_eq!(composed.content().matches("Leaf {{ name }}.").count(), 1, "{}", composed.content());
    assert_eq!(composed.content().matches("Leaf Caller.").count(), 1, "{}", composed.content());
}

#[test]
fn prologue_and_epilogue_of_a_disabled_child_inherit_it() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "intro.md", "Intro {{ name }}.\n");
    write(dir.path(), "outro.md", "Outro {{ name }}.\n");
    write(dir.path(), "child.md", "---\nprologue: ./intro.md\nepilogue: [\"./outro.md\"]\n---\nBody {{ name }}.\n");

    let (composed, report) =
        compose_root(dir.path(), "---\nname: Caller\n---\n::file ./child.md interpolate=false\n", deny_all).unwrap();

    let text = composed.content();
    for line in ["Intro {{ name }}.", "Body {{ name }}.", "Outro {{ name }}."] {
        assert!(text.contains(line), "{line:?} in:\n{text}");
    }
    assert!(report.warnings.is_empty(), "{:?}", messages(&report));
}

/// An expression in active input that composes nested Markdown
/// (`as_markdown`) still evaluates, and the content it composes inherits the
/// disabled setting.
#[test]
fn nested_composition_from_an_active_input_inherits_it() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "child.md",
        "---\nsnippet: \"Nested {{{ name }}}\"\n---\nProse {{ as_markdown(snippet) }}.\n::block when=\"length(as_markdown('A {{ name }}')) > 0\"\nGate ran.\n::end-block\n",
    );

    let (composed, _) =
        compose_root(dir.path(), "---\nname: Caller\n---\n::file ./child.md interpolate=false\n", |options| options).unwrap();

    let text = composed.content();
    assert!(text.contains("Prose {{ as_markdown(snippet) }}."), "prose is never evaluated:\n{text}");
    assert!(text.contains("Gate ran."), "the condition composed its nested Markdown:\n{text}");
}

#[test]
fn a_saved_literal_output_recomposes_under_normal_rules() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "child.md", "Hello {{ name }}.\n");
    let (composed, _) =
        compose_root(dir.path(), "---\nname: World\n---\n::file ./child.md interpolate=false\n", |options| options).unwrap();
    assert!(composed.content().contains("Hello {{ name }}."));

    let (again, _) =
        compose_root(dir.path(), &format!("---\nname: World\n---\n{}", composed.content()), |options| options).unwrap();
    assert!(again.content().contains("Hello World."), "no hidden setting survives: {}", again.content());
}

/// A read-side sample in suppressed prose reads nothing; the same read in a
/// normal inclusion is in the read set.
#[test]
fn a_read_side_sample_in_suppressed_prose_reads_nothing() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "other.md", "---\ntitle: Other\n---\nBody\n");
    write(dir.path(), "child.md", "Title: {{ frontmatter(\"./other.md\", \"title\") }}.\n");

    let (composed, report) = compose_root(dir.path(), "::file ./child.md interpolate=false\n", deny_all).unwrap();
    assert!(composed.content().contains("Title: {{ frontmatter(\"./other.md\", \"title\") }}."), "{}", composed.content());
    assert!(report.read_set.is_empty(), "{:?}", report.read_set);

    let (composed, report) = compose_root(dir.path(), "::file ./child.md\n", |options| options).unwrap();
    assert!(composed.content().contains("Title: Other."), "{}", composed.content());
    assert!(report.read_set.iter().any(|read| read.ends_with("other.md")), "the control reads: {:?}", report.read_set);
}

/// The call check skips suppressed prose but still rejects an invalid call in
/// a directive input of the same child.
#[test]
fn the_call_check_still_rejects_an_invalid_call_in_a_directive_input() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "leaf.md", "Leaf.\n");
    write(dir.path(), "child.md", "Prose {{ length() }}.\n::file ./leaf.md set.n=\"{{ length() }}\"\n");

    let error = compose_root(dir.path(), "::file ./child.md interpolate=false\n", |options| options).unwrap_err();
    assert!(format!("{error:?}").contains("length"), "{error:?}");
}

/// Pre-flight planning names no ICMP probe for a sample in suppressed prose,
/// and still plans one a directive input of the same child reaches.
#[test]
fn preflight_plans_no_probe_for_a_sample_in_suppressed_prose() {
    use crate::markdown::compose::icmp::IcmpAuthority;
    use crate::markdown::compose::icmp::test_support::{ProbeLog, ScriptedProbe, reply};

    let dir = tempfile::tempdir().unwrap();
    let log = ProbeLog::default();
    let planned = |child: &str| {
        write(dir.path(), "child.md", child);
        let root = dir.path().join("root.md");
        std::fs::write(&root, "::file ./child.md interpolate=false\n").unwrap();
        let mut options = ComposeOptions::new().with_source_file(&root).with_allowed_host("10.1.0.0/16");
        options.icmp = IcmpAuthority::default().with_transport(ScriptedProbe::new(&log, vec![reply(4)]));
        let report = Markdown::try_from(root.as_path())
            .unwrap()
            .compose_preflight(&crate::markdown::compose::test_request(options))
            .expect("preflight succeeds");
        report.icmp_probes.iter().map(|probe| probe.target.to_string()).collect::<Vec<_>>()
    };

    assert!(planned("Probe {{ ping(\"10.1.2.3\") }}.\n").is_empty());
    write(dir.path(), "leaf.md", "Leaf.\n");
    assert_eq!(planned("Up {{ ping(\"10.1.2.3\") }}.\n::file ./leaf.md set.up=\"{{ ping('10.1.2.4') }}\"\n"), ["10.1.2.4"]);
    assert_eq!(log.count(), 0, "planning sends nothing");
}

// ── Pre-flight, caching, and reference-graph agreement ──────────────

/// Options that let `::shell` run with `handler` recording each approval.
fn shell_options(home: &Path, handler: Arc<RecordingApproval>) -> impl FnOnce(ComposeOptions) -> ComposeOptions {
    let config = crate::markdown::compose::shell_expansion::test_shell_config(home, &[]);
    move |options: ComposeOptions| {
        options
            .with_config_snapshot(config)
            .with_shell(ShellExpansionOptions { approval_handler: Some(handler), ..Default::default() })
    }
}

/// The approval walk's command set for `root_text`, sorted.
fn preflight_approvals(dir: &Path, home: &Path, root_text: &str) -> Vec<String> {
    let root = dir.join("root.md");
    std::fs::write(&root, root_text).unwrap();
    let handler = Arc::new(RecordingApproval(Mutex::new(Vec::new())));
    let options = shell_options(home, handler)(ComposeOptions::new().with_source_file(&root));
    let mut approvals = Markdown::try_from(root.as_path())
        .unwrap()
        .compose_preflight(&crate::markdown::compose::test_request(options))
        .expect("preflight succeeds")
        .approval_set();
    approvals.sort();
    approvals
}

/// A child whose prose shows command-like samples (inline code, a fenced
/// `::shell`, and a prose expression that would emit a directive) beside
/// real, interpolated shell inputs.
const SHELL_CHILD: &str = "---\nword: child-word\n---\nProse {{ word }}; run `::shell rm -rf ./tmp` by hand.\n\n```md\n::shell echo fenced\n```\n\n::shell echo {{ word }}\n\n::shell-block\necho block-{{ word }}\n::end-block\n";

/// The approval walk discovers exactly the commands execution runs under a
/// disabled-prose child, and the same set as a normal inclusion when only
/// prose differs.
#[test]
fn the_approval_walk_approves_exactly_the_commands_execution_runs() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    write(dir.path(), "child.md", SHELL_CHILD);

    let disabled = preflight_approvals(dir.path(), home.path(), "::file ./child.md interpolate=false\n");
    let normal = preflight_approvals(dir.path(), home.path(), "::file ./child.md\n");
    assert_eq!(disabled, normal, "only prose differs, so the approval sets agree");
    assert!(disabled.iter().any(|command| command == "echo child-word"), "{disabled:?}");
    assert!(disabled.iter().all(|command| !command.contains("{{") && !command.contains("fenced") && !command.contains("rm ")), "{disabled:?}");

    let recorded = Arc::new(RecordingApproval(Mutex::new(Vec::new())));
    let (composed, _) =
        compose_root(dir.path(), "::file ./child.md interpolate=false\n", shell_options(home.path(), Arc::clone(&recorded))).unwrap();
    assert!(composed.content().contains("Prose {{ word }}"), "{}", composed.content());
    let mut executed = recorded.0.lock().unwrap().clone();
    executed.sort();
    assert_eq!(executed, disabled, "execution asks for exactly the walked commands");
}

/// A prose expression that would compose a `::shell` directive is evaluated
/// by neither the walk nor execution under `false`; a normal inclusion
/// discovers and runs it.
#[test]
fn a_command_sample_in_suppressed_prose_is_never_discovered() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    write(dir.path(), "child.md", "Sample {{ as_markdown(\"::shell echo sample\") }}.\n\n::shell echo real\n");

    assert_eq!(preflight_approvals(dir.path(), home.path(), "::file ./child.md interpolate=false\n"), ["echo real"]);
    assert_eq!(preflight_approvals(dir.path(), home.path(), "::file ./child.md\n"), ["echo real", "echo sample"]);

    let recorded = Arc::new(RecordingApproval(Mutex::new(Vec::new())));
    let (composed, _) =
        compose_root(dir.path(), "::file ./child.md interpolate=false\n", shell_options(home.path(), Arc::clone(&recorded))).unwrap();
    assert!(composed.content().contains("Sample {{ as_markdown("), "{}", composed.content());
    assert_eq!(*recorded.0.lock().unwrap(), ["echo real"]);
}

/// One child included twice, once with `interpolate=false`, is walked under
/// each setting in either order: the enabled copy's prose sample is
/// approved, and execution runs exactly the walked commands.
#[test]
fn a_child_included_under_both_settings_is_walked_under_each() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    write(dir.path(), "child.md", "Sample {{ as_markdown(\"::shell echo sample\") }}.\n");

    for root in ["::file ./child.md interpolate=false\n\n::file ./child.md\n", "::file ./child.md\n\n::file ./child.md interpolate=false\n"] {
        assert_eq!(preflight_approvals(dir.path(), home.path(), root), ["echo sample"], "{root:?}");
        let recorded = Arc::new(RecordingApproval(Mutex::new(Vec::new())));
        let (composed, _) = compose_root(dir.path(), root, shell_options(home.path(), Arc::clone(&recorded))).unwrap();
        assert!(composed.content().contains("Sample {{ as_markdown("), "{root:?}: {}", composed.content());
        assert_eq!(*recorded.0.lock().unwrap(), ["echo sample"], "{root:?}");
    }
}

/// Copies of one repeated `::file` site that differ only in `interpolate`
/// are walked as distinct parameters, each under its own setting: only the
/// enabled copy's prose sample is discovered, and execution accepts both
/// copies and runs exactly what the walk found.
#[test]
fn repeated_copies_differing_only_in_interpolate_are_distinct_sites() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "child.md",
        "---\nword: w\n---\nCopy {{ word }}. {{ as_markdown(\"::shell echo sample\") }}\n\n::shell echo {{ word }}\n",
    );
    let root_text = "---\n$schema:\n  modes: string[](required)\nmodes: [\"true\", \"false\"]\n---\n::map modes \"item => item\"\n::file ./child.md interpolate={{ item }}\n::end-map\n";

    let root = dir.path().join("root.md");
    std::fs::write(&root, root_text).unwrap();
    let handler = Arc::new(RecordingApproval(Mutex::new(Vec::new())));
    let options = shell_options(home.path(), handler)(ComposeOptions::new().with_source_file(&root));
    let walk = Markdown::try_from(root.as_path())
        .unwrap()
        .compose_preflight(&crate::markdown::compose::test_request(options))
        .expect("preflight succeeds");
    let copies: Vec<(u64, bool)> = walk
        .preflight_graph
        .edges
        .iter()
        .map(|edge| (edge.parameters, edge.directive.options.interpolate))
        .collect();
    assert_eq!(copies.len(), 2, "one edge per copy: {copies:?}");
    assert_eq!(copies[0], (0, true), "the enabled copy keeps the plain identity");
    assert!(copies[1].0 != 0 && !copies[1].1, "the disabled copy is distinct: {copies:?}");
    let mut walked = walk.approval_set();
    walked.sort();
    walked.dedup();
    assert_eq!(walked, ["echo sample", "echo w"]);

    let recorded = Arc::new(RecordingApproval(Mutex::new(Vec::new())));
    let (composed, _) = compose_root(dir.path(), root_text, shell_options(home.path(), Arc::clone(&recorded))).unwrap();
    let text = composed.content();
    assert!(text.contains("Copy w.") && text.contains("Copy {{ word }}. {{ as_markdown("), "each copy has its own setting:\n{text}");
    let mut executed = recorded.0.lock().unwrap().clone();
    executed.sort();
    executed.dedup();
    assert_eq!(executed, walked);
}

/// Within one run the composed-child cache serves a repeated inclusion only
/// under the same effective setting: a normal and a disabled inclusion of
/// one file never share an entry, in either order.
#[test]
fn a_composed_child_is_reused_only_under_the_same_setting() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "child.md", "Hello {{ name }}.\n");
    let hits = |directives: [&str; 2]| {
        let root = format!("---\nname: World\n---\n{}\n\n{}\n", directives[0], directives[1]);
        let (composed, report) =
            compose_root(dir.path(), &root, |options| options.with_cache_access_mode(CacheAccessMode::ReadWrite)).unwrap();
        (composed.content().to_string(), report.cache_stats.expect("the cache is on").hits)
    };
    let normal = "::file ./child.md";
    let disabled = "::file ./child.md interpolate=false";

    let (_, same_normal) = hits([normal, normal]);
    let (_, same_disabled) = hits([disabled, disabled]);
    assert!(same_normal >= 1 && same_disabled >= 1, "control: a repeat under one setting is reused ({same_normal}, {same_disabled})");
    for order in [[normal, disabled], [disabled, normal]] {
        let (text, mixed) = hits(order);
        assert_eq!(mixed, 0, "{order:?}: an entry from one setting is never served to the other");
        assert!(text.contains("Hello World.") && text.contains("Hello {{ name }}."), "{order:?}:\n{text}");
    }
}

/// One tree through execution, the approval walk, and reference-graph
/// preparation: all three keep the child's prose literal, none reports a
/// warning from it, and the walk's commands are the ones execution runs.
#[test]
fn execution_preflight_and_reference_graph_agree() {
    use crate::markdown::reference::{ReferenceGraphOptions, ReferenceKind};

    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "child.md",
        "---\nword: child-word\n---\nSee [{{ undeclared }}](./{{word}}.md). Malformed {{ 1 + }}.\n\n::shell echo {{ word }}\n",
    );
    let root_text = "---\nword: parent\n---\n::file ./child.md interpolate=false\n";

    let recorded = Arc::new(RecordingApproval(Mutex::new(Vec::new())));
    let (composed, report) = compose_root(dir.path(), root_text, |options| {
        deny_all(shell_options(home.path(), Arc::clone(&recorded))(options))
    })
    .unwrap();
    assert!(composed.content().contains("See [{{ undeclared }}](./{{word}}.md). Malformed {{ 1 + }}."), "{}", composed.content());
    assert!(report.warnings.is_empty(), "{:?}", messages(&report));

    let walked = preflight_approvals(dir.path(), home.path(), root_text);
    assert_eq!(walked, ["echo child-word"]);
    assert_eq!(*recorded.0.lock().unwrap(), walked);

    let root = dir.path().join("root.md");
    let options = deny_all(ComposeOptions::new().with_source_file(&root).disable(ComposeOperation::ShellExpansion));
    let graph = Markdown::try_from(root.as_path())
        .unwrap()
        .reference_graph(ReferenceGraphOptions::with_compose(&crate::markdown::compose::test_request(options)))
        .expect("suppressed prose cannot fail preparation");
    assert!(graph.earlier_warnings().is_empty(), "{:?}", graph.earlier_warnings());
    let child_links: Vec<String> = graph
        .nodes()
        .iter()
        .flat_map(|node| node.local_references.by_kind(ReferenceKind::Hyperlink))
        .map(|record| record.target.raw().unwrap().to_string())
        .collect();
    assert_eq!(child_links, ["./{{word}}.md"], "the graph prepares the same literal prose");
}

/// An expression-valued `interpolate=` (each authored spelling) is evaluated
/// first and the result validated: composition, the approval walk, and
/// reference preparation all follow the frontmatter flag.
#[test]
fn an_expression_valued_option_follows_its_result_on_every_path() {
    use crate::markdown::reference::ReferenceGraphOptions;

    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    write(dir.path(), "child.md", "Hello {{ name }}. {{ as_markdown(\"::shell echo sample\") }}\n");
    let spellings = ["interpolate={{ flag }}", "interpolate={{flag}}", "interpolate=\"{{ flag }}\"", "interpolate=\"{{ fixed }}\""];
    for (flag, disabled) in [("false", true), ("true", false)] {
        for spelling in spellings {
            let fixed = spelling.replace("fixed", flag);
            let root_text = format!("---\nname: World\nflag: {flag}\n---\n::file ./child.md {fixed}\n");
            let case = format!("flag {flag}, `{fixed}`");

            let recorded = Arc::new(RecordingApproval(Mutex::new(Vec::new())));
            let (composed, _) =
                compose_root(dir.path(), &root_text, shell_options(home.path(), Arc::clone(&recorded))).unwrap_or_else(|error| panic!("{case}: {error}"));
            let text = composed.content();
            let expected_commands: &[&str] = if disabled { &[] } else { &["echo sample"] };
            assert_eq!(text.contains("Hello {{ name }}."), disabled, "{case}:\n{text}");
            assert_eq!(text.contains("Hello World."), !disabled, "{case}:\n{text}");
            assert_eq!(*recorded.0.lock().unwrap(), expected_commands, "{case}: execution");
            assert_eq!(preflight_approvals(dir.path(), home.path(), &root_text), expected_commands, "{case}: the walk");

            let root = dir.path().join("root.md");
            let options = ComposeOptions::new().with_source_file(&root).disable(ComposeOperation::ShellExpansion);
            Markdown::try_from(root.as_path())
                .unwrap()
                .reference_graph(ReferenceGraphOptions::with_compose(&crate::markdown::compose::test_request(options)))
                .unwrap_or_else(|error| panic!("{case}: reference preparation: {error}"));
        }
    }
}

/// An option expression whose result is not a boolean word is a located
/// composition error naming the evaluated value; the editor cannot know the
/// result and stays silent.
#[test]
fn an_option_expression_with_a_non_boolean_result_fails_at_its_line() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "child.md", "Hello.\n");
    let error = compose_root(dir.path(), "Intro.\n\n::file ./child.md interpolate={{ \"banana\" }}\n", |options| options)
        .expect_err("a non-boolean result is rejected")
        .to_string();
    assert!(error.contains("'interpolate=' must be true or false"), "{error}");
    assert!(error.contains("not banana"), "the evaluated value is named: {error}");
    assert!(error.contains("at line 3"), "the directive's line is located: {error}");
}

mod remote {
    use super::*;
    use crate::markdown::compose::remote::RemoteReadConfig;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn permitted_remote_markdown_inherits_it() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/guide.md"))
            .respond_with(ResponseTemplate::new(200).set_body_string("Remote {{ name }} and {{{ x }}}.\n"))
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let url = format!("{}/guide.md", server.uri());

        for keyword in ["::file", "::url"] {
            let root = format!("---\nname: Caller\n---\n{keyword} {url} interpolate=false\n");
            let (composed, report) = compose_root(dir.path(), &root, |options| {
                deny_all(options)
                    .with_allow_remote_transclusion(true)
                    .with_remote_read_config(RemoteReadConfig { allowed_hosts: vec!["127.0.0.1".into()], ..Default::default() })
            })
            .unwrap();
            assert!(composed.content().contains("Remote {{ name }} and {{{ x }}}."), "{keyword}: {}", composed.content());
            assert!(report.warnings.is_empty(), "{keyword}: {:?}", messages(&report));
        }
    }

    /// A remote sample in suppressed prose is never fetched.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_remote_sample_in_suppressed_prose_is_never_fetched() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/sample.md"))
            .respond_with(ResponseTemplate::new(200).set_body_string("---\nt: T\n---\n"))
            .expect(0)
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "child.md", &format!("Sample {{{{ frontmatter(\"{}/sample.md\", \"t\") }}}}.\n", server.uri()));

        let (composed, report) = compose_root(dir.path(), "::file ./child.md interpolate=false\n", |options| {
            deny_all(options)
                .with_allow_remote_transclusion(true)
                .with_remote_read_config(RemoteReadConfig { allowed_hosts: vec!["127.0.0.1".into()], ..Default::default() })
        })
        .unwrap();
        assert!(composed.content().contains("Sample {{ frontmatter("), "{}", composed.content());
        assert_eq!(report.remote_fetch_stats.map(|stats| stats.fetched), Some(0));
        assert!(report.read_set.is_empty(), "{:?}", report.read_set);
    }

    /// A mapped remote target is fetched once per item, as evaluated; its
    /// unevaluated template spelling is never prefetched.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_mapped_remote_target_fetches_only_each_items_url() {
        let server = MockServer::start().await;
        for (name, body) in [("a", "Alpha {{ name }}.\n"), ("b", "Beta {{ name }}.\n")] {
            Mock::given(method("GET"))
                .and(path(format!("/{name}.md")))
                .respond_with(ResponseTemplate::new(200).set_body_string(body))
                .mount(&server)
                .await;
        }
        let dir = tempfile::tempdir().unwrap();
        let root = format!(
            "---\n$schema:\n  words: string[](required)\nwords: [a, b]\n---\n::map words \"item => item\"\n::file {}/{{{{ item }}}}.md interpolate=false\n::end-map\n",
            server.uri()
        );

        let (composed, report) = compose_root(dir.path(), &root, |options| {
            deny_all(options)
                .with_allow_remote_transclusion(true)
                .with_remote_read_config(RemoteReadConfig { allowed_hosts: vec!["127.0.0.1".into()], ..Default::default() })
        })
        .unwrap();
        assert!(composed.content().contains("Alpha {{ name }}."), "{}", composed.content());
        assert!(composed.content().contains("Beta {{ name }}."), "{}", composed.content());
        assert!(report.warnings.is_empty(), "{:?}", messages(&report));
        let mut paths: Vec<String> =
            server.received_requests().await.unwrap().iter().map(|request| request.url.path().to_string()).collect();
        paths.sort();
        assert_eq!(paths, ["/a.md", "/b.md"]);
    }

    // ── Pre-flight over a remote child (`::file` and `::url`) ───────────
    //
    // The walk derives a remote child's request on its own path; each case
    // runs the walk and composition under `interpolate=false`, then the same
    // root without the option as the control that proves the walk reads the
    // remote prose at all.

    async fn serve(body: &str) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/sample.md"))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .mount(&server)
            .await;
        server
    }

    fn remote_allowed(options: ComposeOptions) -> ComposeOptions {
        options
            .with_allow_remote_transclusion(true)
            .with_remote_read_config(RemoteReadConfig { allowed_hosts: vec!["127.0.0.1".into()], ..Default::default() })
    }

    /// `root.md` including the served sample with `keyword`, with or
    /// without `interpolate=false`.
    fn remote_root(server: &MockServer, keyword: &str, disabled: bool) -> String {
        let option = if disabled { " interpolate=false" } else { "" };
        format!("{keyword} {}/sample.md{option}\n", server.uri())
    }

    fn preflight_remote(
        dir: &Path,
        root_text: &str,
        configure: impl FnOnce(ComposeOptions) -> ComposeOptions,
    ) -> MarkdownResult<crate::markdown::compose::ComposePreflightReport> {
        let root = dir.join("root.md");
        std::fs::write(&root, root_text).unwrap();
        let options = configure(remote_allowed(ComposeOptions::new().with_source_file(&root)));
        Markdown::try_from(root.as_path()).unwrap().compose_preflight(&crate::markdown::compose::test_request(options))
    }

    /// A proven-bad call in suppressed remote prose is rejected by neither
    /// the walk nor execution, even under deny-all.
    async fn call_check_case(keyword: &str) {
        let server = serve("Sample {{ length() }}.\n").await;
        let dir = tempfile::tempdir().unwrap();

        preflight_remote(dir.path(), &remote_root(&server, keyword, true), deny_all).expect("the walk accepts the sample");
        let (composed, report) =
            compose_root(dir.path(), &remote_root(&server, keyword, true), |options| deny_all(remote_allowed(options)))
                .unwrap();
        assert!(composed.content().contains("Sample {{ length() }}."), "{keyword}: {}", composed.content());
        assert!(report.warnings.is_empty(), "{keyword}: {:?}", messages(&report));

        let control = preflight_remote(dir.path(), &remote_root(&server, keyword, false), deny_all).unwrap_err();
        assert!(format!("{control:?}").contains("length"), "{keyword}: {control:?}");
    }

    /// A prose `as_markdown` sample in suppressed remote prose adds no
    /// command to the approval set, through either preflight API, and
    /// execution runs none.
    async fn nested_command_case(keyword: &str) {
        let server = serve("Sample {{ as_markdown(\"::shell echo sample\") }}.\n").await;
        let dir = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let approvals = |disabled: bool| {
            let handler = Arc::new(RecordingApproval(Mutex::new(Vec::new())));
            let report =
                preflight_remote(dir.path(), &remote_root(&server, keyword, disabled), shell_options(home.path(), handler))
                    .expect("preflight succeeds");
            let root = dir.path().join("root.md");
            let handler = Arc::new(RecordingApproval(Mutex::new(Vec::new())));
            let options = remote_allowed(shell_options(home.path(), Arc::clone(&handler))(
                ComposeOptions::new().with_source_file(&root),
            ));
            let gated = Markdown::try_from(root.as_path())
                .unwrap()
                .compose_preflight_approvals(&crate::markdown::compose::test_request(options), Some(handler))
                .expect("approval walk succeeds");
            let mut gated: Vec<String> = gated.pre_approved_commands.exact().cloned().collect();
            gated.sort();
            (report.approval_set(), gated)
        };

        assert_eq!(approvals(true), (Vec::<String>::new(), Vec::new()), "{keyword}");
        let recorded = Arc::new(RecordingApproval(Mutex::new(Vec::new())));
        let (composed, _) = compose_root(dir.path(), &remote_root(&server, keyword, true), |options| {
            remote_allowed(shell_options(home.path(), Arc::clone(&recorded))(options))
        })
        .unwrap();
        assert!(composed.content().contains("Sample {{ as_markdown("), "{keyword}: {}", composed.content());
        assert!(recorded.0.lock().unwrap().is_empty(), "{keyword}: {:?}", recorded.0.lock().unwrap());

        let (control, control_gated) = approvals(false);
        assert_eq!(control, ["echo sample"], "{keyword}");
        assert_eq!(control_gated, ["echo sample"], "{keyword}");
    }

    /// Lazy reads and probes in suppressed remote prose report no deferred
    /// key, no capability, and no planned probe; nothing is ever sent.
    async fn deferred_capability_case(keyword: &str) {
        use crate::markdown::compose::icmp::IcmpAuthority;
        use crate::markdown::compose::icmp::test_support::{ProbeLog, ScriptedProbe, reply};

        let server = serve("Sample {{ current.os }} and {{ ping('10.1.2.3') }}.\n").await;
        let dir = tempfile::tempdir().unwrap();
        let log = ProbeLog::default();
        let scripted = |options: ComposeOptions| {
            let mut options = options.with_allowed_host("10.1.0.0/16");
            options.icmp = IcmpAuthority::default().with_transport(ScriptedProbe::new(&log, vec![reply(4)]));
            options
        };
        let observed = |disabled: bool| {
            let report = preflight_remote(dir.path(), &remote_root(&server, keyword, disabled), scripted).unwrap();
            let keys: Vec<String> = report.deferred_context.current_keys().map(str::to_string).collect();
            let functions: Vec<String> = report.deferred_context.functions().map(str::to_string).collect();
            let probes: Vec<String> = report.icmp_probes.iter().map(|probe| probe.target.to_string()).collect();
            (keys, functions, probes)
        };

        let (keys, functions, probes) = observed(true);
        assert!(!keys.iter().any(|key| key == "os"), "{keyword}: {keys:?}");
        assert!(!functions.iter().any(|name| name == "ping"), "{keyword}: {functions:?}");
        assert!(probes.is_empty(), "{keyword}: {probes:?}");
        let (composed, _) =
            compose_root(dir.path(), &remote_root(&server, keyword, true), |options| scripted(remote_allowed(options)))
                .unwrap();
        assert!(composed.content().contains("Sample {{ current.os }} and {{ ping('10.1.2.3') }}."), "{keyword}: {}", composed.content());

        let (keys, functions, probes) = observed(false);
        assert!(keys.iter().any(|key| key == "os"), "{keyword}: {keys:?}");
        assert!(functions.iter().any(|name| name == "ping"), "{keyword}: {functions:?}");
        assert_eq!(probes, ["10.1.2.3"], "{keyword}");
        assert_eq!(log.count(), 0, "{keyword}: planning and suppressed prose send nothing");
    }

    /// One remote child included twice, once with `interpolate=false`, is
    /// walked under each setting in either order.
    async fn repeated_inclusion_case(keyword: &str) {
        let server = serve("Sample {{ as_markdown(\"::shell echo sample\") }}.\n").await;
        let dir = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let (off, on) = (remote_root(&server, keyword, true), remote_root(&server, keyword, false));

        for root in [format!("{off}\n{on}"), format!("{on}\n{off}")] {
            let handler = Arc::new(RecordingApproval(Mutex::new(Vec::new())));
            let walked = preflight_remote(dir.path(), &root, shell_options(home.path(), handler)).unwrap().approval_set();
            assert_eq!(walked, ["echo sample"], "{keyword} {root:?}");
            let recorded = Arc::new(RecordingApproval(Mutex::new(Vec::new())));
            compose_root(dir.path(), &root, |options| remote_allowed(shell_options(home.path(), Arc::clone(&recorded))(options)))
                .unwrap();
            assert_eq!(*recorded.0.lock().unwrap(), ["echo sample"], "{keyword} {root:?}");
        }
    }

    macro_rules! remote_preflight_cases {
        ($($spelling:ident => $keyword:literal),* $(,)?) => {$(
            mod $spelling {
                #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
                async fn the_call_check_accepts_a_suppressed_sample() {
                    super::call_check_case($keyword).await;
                }

                #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
                async fn nested_command_discovery_skips_a_suppressed_sample() {
                    super::nested_command_case($keyword).await;
                }

                #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
                async fn capability_and_probe_discovery_skip_a_suppressed_sample() {
                    super::deferred_capability_case($keyword).await;
                }

                #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
                async fn a_repeated_inclusion_is_walked_under_each_setting() {
                    super::repeated_inclusion_case($keyword).await;
                }
            }
        )*};
    }

    remote_preflight_cases!(file_directive => "::file", url_directive => "::url");
}

// ── `set` overlays on every path that loads a child ─────────────────
//
// Each consumer that loads an included child (execution, the approval walk,
// reference preparation; local, remote `::file`, remote `::url`) must load
// it with the directive's `set` values written over its frontmatter, so a
// condition or command reads the same value on every path. Each case runs
// with no `set` (the control), the property form, and the object form, with
// and without `interpolate=false`.
mod set_overlay {
    use super::*;
    use crate::markdown::compose::remote::RemoteReadConfig;
    use crate::markdown::reference::{ReferenceGraphOptions, ReferenceKind};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// A child whose `word` selects one of two links.
    const LINK_CHILD: &str = "---\nword: default\n---\n::block when=\"word == 'supplied'\"\n[Selected](./supplied.md)\n::end-block\n::block when=\"word == 'default'\"\n[Default](./default.md)\n::end-block\n";

    /// A child whose `word` is a command's argument.
    const SHELL_CHILD: &str = "---\nword: default\n---\n::shell echo {{ word }}\n";

    /// The link text the composed child keeps, and the one it removes, when
    /// `word` is `expected` (the root normalizes the targets' spelling).
    fn link_for(expected: &str) -> (&'static str, &'static str) {
        if expected == "supplied" { ("[Selected](", "[Default](") } else { ("[Default](", "[Selected](") }
    }

    fn local_execution_case(options: &str, expected: &str) {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "child.md", LINK_CHILD);
        let (composed, _) = compose_root(dir.path(), &format!("::file ./child.md{options}\n"), |options| options).unwrap();
        let (kept, removed) = link_for(expected);
        assert!(composed.content().contains(kept), "{options:?}: {}", composed.content());
        assert!(!composed.content().contains(removed), "{options:?}: {}", composed.content());
    }

    fn local_preflight_case(options: &str, expected: &str) {
        let dir = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        write(dir.path(), "child.md", SHELL_CHILD);
        let root_text = format!("::file ./child.md{options}\n");
        let command = format!("echo {expected}");
        assert_eq!(preflight_approvals(dir.path(), home.path(), &root_text), std::slice::from_ref(&command), "{options:?}");
        let recorded = Arc::new(RecordingApproval(Mutex::new(Vec::new())));
        compose_root(dir.path(), &root_text, shell_options(home.path(), Arc::clone(&recorded))).unwrap();
        assert_eq!(*recorded.0.lock().unwrap(), [command], "{options:?}: execution runs the walked bytes");
    }

    /// Reference preparation keeps the branch composition keeps, and the
    /// reference check passes with only that branch's target present.
    fn local_reference_case(options: &str, expected: &str) {
        use crate::markdown::reference::validate::{ReferenceSeverity, ReferenceValidationOptions};

        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "child.md", LINK_CHILD);
        write(dir.path(), &format!("{expected}.md"), "# Target\n");
        let root = dir.path().join("root.md");
        std::fs::write(&root, format!("::file ./child.md{options}\n")).unwrap();
        let request = crate::markdown::compose::test_request(
            ComposeOptions::new().with_source_file(&root).for_reference_check(),
        );
        let md = Markdown::try_from(root.as_path()).unwrap();

        let graph = md.reference_graph(ReferenceGraphOptions::with_compose(&request)).unwrap();
        let links: Vec<String> = graph
            .nodes()
            .iter()
            .flat_map(|node| node.local_references.by_kind(ReferenceKind::Hyperlink))
            .map(|record| record.target.raw().unwrap().to_string())
            .collect();
        assert_eq!(links, [format!("./{expected}.md")], "{options:?}");

        let report = md
            .validate_references(ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(&request)))
            .unwrap();
        let errors: Vec<_> = report.issues.iter().filter(|issue| issue.severity == ReferenceSeverity::Error).collect();
        assert!(errors.is_empty(), "{options:?}: {errors:?}");
    }

    async fn serve(body: &str) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/child.md"))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .mount(&server)
            .await;
        server
    }

    fn remote_allowed(options: ComposeOptions) -> ComposeOptions {
        options
            .with_allow_remote_transclusion(true)
            .with_remote_read_config(RemoteReadConfig { allowed_hosts: vec!["127.0.0.1".into()], ..Default::default() })
    }

    async fn remote_execution_case(keyword: &str, options: &str, expected: &str) {
        let server = serve(LINK_CHILD).await;
        let dir = tempfile::tempdir().unwrap();
        let root_text = format!("{keyword} {}/child.md{options}\n", server.uri());
        let (composed, _) = compose_root(dir.path(), &root_text, remote_allowed).unwrap();
        let (kept, removed) = link_for(expected);
        assert!(composed.content().contains(kept), "{keyword}{options:?}: {}", composed.content());
        assert!(!composed.content().contains(removed), "{keyword}{options:?}: {}", composed.content());
    }

    async fn remote_preflight_case(keyword: &str, options: &str, expected: &str) {
        let server = serve(SHELL_CHILD).await;
        let dir = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let root_text = format!("{keyword} {}/child.md{options}\n", server.uri());
        std::fs::write(&root, &root_text).unwrap();
        let command = format!("echo {expected}");

        let handler = Arc::new(RecordingApproval(Mutex::new(Vec::new())));
        let request_options = remote_allowed(shell_options(home.path(), handler)(ComposeOptions::new().with_source_file(&root)));
        let walked = Markdown::try_from(root.as_path())
            .unwrap()
            .compose_preflight(&crate::markdown::compose::test_request(request_options))
            .expect("preflight succeeds")
            .approval_set();
        assert_eq!(walked, std::slice::from_ref(&command), "{keyword}{options:?}");

        let recorded = Arc::new(RecordingApproval(Mutex::new(Vec::new())));
        compose_root(dir.path(), &root_text, |options| remote_allowed(shell_options(home.path(), Arc::clone(&recorded))(options)))
            .unwrap();
        assert_eq!(*recorded.0.lock().unwrap(), [command], "{keyword}{options:?}: execution runs the walked bytes");
    }

    /// One child included twice with different `set` values is walked once
    /// per value set, as execution composes it once per value set.
    #[test]
    fn a_child_included_with_two_value_sets_is_walked_under_each() {
        let dir = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        write(dir.path(), "child.md", SHELL_CHILD);
        let root_text = "::file ./child.md set.word=\"one\"\n\n::file ./child.md set='{word: \"two\"}'\n\n::file ./child.md\n";
        let expected = ["echo default", "echo one", "echo two"];
        assert_eq!(preflight_approvals(dir.path(), home.path(), root_text), expected);
        let recorded = Arc::new(RecordingApproval(Mutex::new(Vec::new())));
        compose_root(dir.path(), root_text, shell_options(home.path(), Arc::clone(&recorded))).unwrap();
        let mut executed = recorded.0.lock().unwrap().clone();
        executed.sort();
        assert_eq!(executed, expected);
    }

    /// A document that includes itself with a new value on every level ends
    /// its walk at the cycle (composition rejects the cycle) instead of
    /// walking each new value set.
    #[test]
    fn a_self_inclusion_with_growing_values_ends_at_the_cycle() {
        let dir = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let root_text = "---\nn: 0\n---\n::file ./root.md set.n={{ n + 1 }}\n\n::shell echo {{ n }}\n";
        assert_eq!(preflight_approvals(dir.path(), home.path(), root_text), ["echo 0"]);
    }

    async fn remote_value_sets_case(keyword: &str) {
        let server = serve(SHELL_CHILD).await;
        let dir = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let url = format!("{}/child.md", server.uri());
        let root_text = format!("{keyword} {url} set.word=\"one\"\n\n{keyword} {url} set='{{word: \"two\"}}'\n\n{keyword} {url}\n");
        std::fs::write(&root, &root_text).unwrap();
        let expected = ["echo default", "echo one", "echo two"];

        let handler = Arc::new(RecordingApproval(Mutex::new(Vec::new())));
        let request_options = remote_allowed(shell_options(home.path(), handler)(ComposeOptions::new().with_source_file(&root)));
        let mut walked = Markdown::try_from(root.as_path())
            .unwrap()
            .compose_preflight(&crate::markdown::compose::test_request(request_options))
            .expect("preflight succeeds")
            .approval_set();
        walked.sort();
        assert_eq!(walked, expected, "{keyword}");

        let recorded = Arc::new(RecordingApproval(Mutex::new(Vec::new())));
        compose_root(dir.path(), &root_text, |options| remote_allowed(shell_options(home.path(), Arc::clone(&recorded))(options)))
            .unwrap();
        let mut executed = recorded.0.lock().unwrap().clone();
        executed.sort();
        assert_eq!(executed, expected, "{keyword}");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_remote_file_child_included_with_two_value_sets_is_walked_under_each() {
        remote_value_sets_case("::file").await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_remote_url_child_included_with_two_value_sets_is_walked_under_each() {
        remote_value_sets_case("::url").await;
    }

    /// A remote grandchild reached through a local parent included with two
    /// value sets inherits none of the parent's values: discovery and
    /// execution both see only its own default.
    async fn remote_grandchild_case(keyword: &str) {
        let server = serve(SHELL_CHILD).await;
        let dir = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        write(dir.path(), "parent.md", &format!("{keyword} {}/child.md\n", server.uri()));
        let root_text = "::file ./parent.md set.word=\"one\"\n\n::file ./parent.md set.word=\"two\"\n";
        let root = dir.path().join("root.md");
        std::fs::write(&root, root_text).unwrap();

        let handler = Arc::new(RecordingApproval(Mutex::new(Vec::new())));
        let request_options = remote_allowed(shell_options(home.path(), handler)(ComposeOptions::new().with_source_file(&root)));
        let walked = Markdown::try_from(root.as_path())
            .unwrap()
            .compose_preflight(&crate::markdown::compose::test_request(request_options))
            .expect("preflight succeeds")
            .approval_set();
        assert_eq!(walked, ["echo default"], "{keyword}");

        let recorded = Arc::new(RecordingApproval(Mutex::new(Vec::new())));
        compose_root(dir.path(), root_text, |options| remote_allowed(shell_options(home.path(), Arc::clone(&recorded))(options)))
            .unwrap();
        assert!(recorded.0.lock().unwrap().iter().all(|command| command == "echo default"), "{keyword}");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_remote_file_grandchild_inherits_no_parent_values() {
        remote_grandchild_case("::file").await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_remote_url_grandchild_inherits_no_parent_values() {
        remote_grandchild_case("::url").await;
    }

    /// A `set` value an expression inserted is data in a remote child, as in
    /// a local one: the child reads it without scanning it again.
    async fn remote_data_value_case(keyword: &str) {
        let server = serve("---\nactor: default\n---\nActor={{ actor }}\n").await;
        let dir = tempfile::tempdir().unwrap();
        let root_text = format!(
            "---\nx: \"{{{{ '{{{{' + ' area }}}}' }}}}\"\n---\n{keyword} {}/child.md set.actor=\"{{{{ x }}}}\"\n",
            server.uri()
        );
        let (composed, _) = compose_root(dir.path(), &root_text, remote_allowed).unwrap();
        assert!(composed.content().contains("Actor={{ area }}"), "{keyword}: {}", composed.content());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_remote_file_child_reads_a_data_set_value_as_data() {
        remote_data_value_case("::file").await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_remote_url_child_reads_a_data_set_value_as_data() {
        remote_data_value_case("::url").await;
    }

    macro_rules! set_overlay_cases {
        ($($form:ident => ($options:expr, $expected:literal)),* $(,)?) => {$(
            mod $form {
                const OPTIONS: &str = $options;

                #[test]
                fn local_execution() {
                    super::local_execution_case(OPTIONS, $expected);
                }

                #[test]
                fn local_preflight() {
                    super::local_preflight_case(OPTIONS, $expected);
                }

                #[test]
                fn local_reference_preparation() {
                    super::local_reference_case(OPTIONS, $expected);
                }

                #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
                async fn remote_file_execution() {
                    super::remote_execution_case("::file", OPTIONS, $expected).await;
                }

                #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
                async fn remote_url_execution() {
                    super::remote_execution_case("::url", OPTIONS, $expected).await;
                }

                #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
                async fn remote_file_preflight() {
                    super::remote_preflight_case("::file", OPTIONS, $expected).await;
                }

                #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
                async fn remote_url_preflight() {
                    super::remote_preflight_case("::url", OPTIONS, $expected).await;
                }
            }
        )*};
    }

    set_overlay_cases!(
        no_set => ("", "default"),
        property_form => (" set.word=\"supplied\"", "supplied"),
        object_form => (" set='{word: \"supplied\"}'", "supplied"),
        no_set_disabled => (" interpolate=false", "default"),
        property_form_disabled => (" interpolate=false set.word=\"supplied\"", "supplied"),
        object_form_disabled => (" interpolate=false set='{word: \"supplied\"}'", "supplied"),
    );
}

/// A shared fragment reached through a parent included twice with
/// different `set` values: discovery walks the fragment once per inherited
/// value set, as execution composes it once per value set.
mod inherited_values {
    use super::*;
    use crate::markdown::compose::preflight::{ComposePreflightReport, PreflightGraphNode, PreflightResolvedTarget};

    const LEAF: &str = "::shell echo {{ word }}\n";

    /// Root text including `parent.md` once per value in `values`, each
    /// inclusion carrying `options` before its `set`.
    fn root_text(options: &str, values: &[&str]) -> String {
        values
            .iter()
            .map(|value| format!("::file ./parent.md{options} set.word=\"{value}\"\n"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn preflight(dir: &Path, home: &Path, root_text: &str) -> ComposePreflightReport {
        let root = dir.join("root.md");
        std::fs::write(&root, root_text).unwrap();
        let handler = Arc::new(RecordingApproval(Mutex::new(Vec::new())));
        let options = shell_options(home, handler)(ComposeOptions::new().with_source_file(&root));
        Markdown::try_from(root.as_path())
            .unwrap()
            .compose_preflight(&crate::markdown::compose::test_request(options))
            .expect("preflight succeeds")
    }

    fn sorted(mut commands: Vec<String>) -> Vec<String> {
        commands.sort();
        commands
    }

    /// Discovery approves exactly the commands a recording execution asks
    /// for, and execution with only those pre-approved (and the walked graph)
    /// succeeds.
    fn case(parent: &str, leaf: &str, options: &str, expected: &[&str]) {
        let dir = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        write(dir.path(), "parent.md", parent);
        write(dir.path(), "leaf.md", leaf);
        let root_text = root_text(options, &["one", "two"]);

        let report = preflight(dir.path(), home.path(), &root_text);
        let walked = sorted(report.approval_set());
        assert_eq!(walked, expected, "discovery approves every inherited value's command");

        let recorded = Arc::new(RecordingApproval(Mutex::new(Vec::new())));
        compose_root(dir.path(), &root_text, shell_options(home.path(), Arc::clone(&recorded))).unwrap();
        assert_eq!(sorted(recorded.0.lock().unwrap().clone()), walked, "execution asks for exactly the walked commands");

        let config = crate::markdown::compose::shell_expansion::test_shell_config(home.path(), &[]);
        let approved: std::collections::HashSet<String> = walked.iter().cloned().collect();
        let (composed, _) = compose_root(dir.path(), &root_text, |options| {
            options
                .with_config_snapshot(config)
                .with_pre_approved_commands(approved)
                .with_preflight_graph(report.preflight_graph)
        })
        .expect("pre-approved execution accepts every discovered command");
        for command in expected {
            let output = command.trim_start_matches("echo ");
            assert!(composed.content().contains(output), "{output}: {}", composed.content());
        }
    }

    macro_rules! inherited_value_cases {
        ($($name:ident => ($parent:expr, $options:expr)),* $(,)?) => {$(
            #[test]
            fn $name() {
                case($parent, LEAF, $options, &["echo one", "echo two"]);
            }
        )*};
    }

    inherited_value_cases!(
        file_grandchild => ("::file ./leaf.md\n", " interpolate=false"),
        file_grandchild_normal_prose => ("::file ./leaf.md\n", ""),
        prologue => ("---\nprologue: ./leaf.md\n---\nParent.\n", " interpolate=false"),
        epilogue => ("---\nepilogue: ./leaf.md\n---\nParent.\n", " interpolate=false"),
        nested_as_markdown => ("Nested {{ as_markdown(\"::shell echo {{ word }}\") }}.\n", ""),
    );

    /// Composing with exactly the discovered commands pre-approved never
    /// meets a command discovery missed.
    #[test]
    fn pre_approved_execution_accepts_every_discovered_command() {
        let dir = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        write(dir.path(), "parent.md", "::file ./leaf.md\n");
        write(dir.path(), "leaf.md", LEAF);
        let root_text = root_text(" interpolate=false", &["one", "two"]);
        let report = preflight(dir.path(), home.path(), &root_text);
        let config = crate::markdown::compose::shell_expansion::test_shell_config(home.path(), &[]);
        let approved: std::collections::HashSet<String> = report.approval_set().into_iter().collect();
        let result = compose_root(dir.path(), &root_text, |options| {
            options.with_config_snapshot(config).with_pre_approved_commands(approved)
        });
        let (composed, _) = result.unwrap_or_else(|error| panic!("{error}"));
        assert!(composed.content().contains("one") && composed.content().contains("two"), "{}", composed.content());
    }

    /// Every resolved local target edge anywhere in `node`'s subtree.
    fn target_files(node: &PreflightGraphNode, out: &mut Vec<String>) {
        for edge in &node.edges {
            if let PreflightResolvedTarget::File { path, .. } = &edge.resolved_target {
                out.push(path.file_name().unwrap().to_string_lossy().to_string());
            }
            target_files(&edge.child, out);
        }
    }

    /// A grandchild whose target interpolates the inherited value keeps one
    /// resolved edge per value, as the second-only control records its own.
    #[test]
    fn grandchild_targets_keep_each_inherited_value() {
        let dir = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        write(dir.path(), "parent.md", "::file ./leaf.md\n");
        write(dir.path(), "leaf.md", "::file ./{{ word }}.md\n");
        write(dir.path(), "10.1.2.3.md", "::shell echo three\n");
        write(dir.path(), "10.1.2.4.md", "::shell echo four\n");

        let mut second_only = Vec::new();
        target_files(
            &preflight(dir.path(), home.path(), &root_text(" interpolate=false", &["10.1.2.4"])).preflight_graph,
            &mut second_only,
        );
        assert!(second_only.contains(&"10.1.2.4.md".to_string()), "{second_only:?}");

        let report = preflight(dir.path(), home.path(), &root_text(" interpolate=false", &["10.1.2.3", "10.1.2.4"]));
        let mut both = Vec::new();
        target_files(&report.preflight_graph, &mut both);
        assert!(both.contains(&"10.1.2.3.md".to_string()) && both.contains(&"10.1.2.4.md".to_string()), "{both:?}");
        assert_eq!(sorted(report.approval_set()), ["echo four", "echo three"]);
    }
}

/// Reference preparation selects a grandchild's branches from the state
/// composition gives it: values inherited from a parent included with `set`,
/// and the parent's frontmatter after it resolved.
mod reference_inheritance {
    use super::*;
    use crate::markdown::reference::validate::{ReferenceSeverity, ReferenceValidationOptions};
    use crate::markdown::reference::{ReferenceGraphOptions, ReferenceKind};

    /// A leaf whose only link sits behind an inherited `word`.
    const LEAF: &str = "::block when=\"word == 'supplied'\"\n[Selected](./missing.md)\n::end-block\n";

    const ROOT: &str = "::file ./parent.md interpolate=false set.word=\"supplied\"\n";

    /// Writes `root.md` and returns the graph's hyperlink targets and the
    /// reference check's error targets, read through the public graph and
    /// validation entry points with the request `md compose` checks with.
    fn references(dir: &Path, root_text: &str) -> (Vec<String>, Vec<String>) {
        let root = dir.join("root.md");
        std::fs::write(&root, root_text).unwrap();
        let request =
            crate::markdown::compose::test_request(ComposeOptions::new().with_source_file(&root).for_reference_check());
        let md = Markdown::try_from(root.as_path()).unwrap();
        let graph = md.reference_graph(ReferenceGraphOptions::with_compose(&request)).unwrap();
        let links = graph
            .nodes()
            .iter()
            .flat_map(|node| node.local_references.by_kind(ReferenceKind::Hyperlink))
            .map(|record| record.target.raw().unwrap().to_string())
            .collect();
        let report = md
            .validate_references(ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(&request)))
            .unwrap();
        let errors = report
            .issues
            .iter()
            .filter(|issue| issue.severity == ReferenceSeverity::Error)
            .map(|issue| format!("{issue:?}"))
            .collect();
        (links, errors)
    }

    /// Composition keeps the selected link, the graph records it, and the
    /// reference check rejects its missing target.
    fn case(parent: &str, root_text: &str) {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "parent.md", parent);
        write(dir.path(), "leaf.md", LEAF);

        let (composed, _) = compose_root(dir.path(), root_text, |options| options).unwrap();
        assert!(composed.content().contains("[Selected]("), "composition selects the link: {}", composed.content());

        let (links, errors) = references(dir.path(), root_text);
        assert_eq!(links, ["./missing.md"], "the graph records the selected link");
        assert!(
            errors.len() == 1 && errors[0].contains("missing.md"),
            "the reference check rejects the missing target: {errors:?}"
        );
    }

    #[test]
    fn a_body_file_grandchild_inherits_the_supplied_value() {
        case("::file ./leaf.md\n", ROOT);
    }

    #[test]
    fn a_prologue_grandchild_inherits_the_supplied_value() {
        case("---\nprologue: ./leaf.md\n---\nParent.\n", ROOT);
    }

    #[test]
    fn an_epilogue_grandchild_inherits_the_supplied_value() {
        case("---\nepilogue: ./leaf.md\n---\nParent.\n", ROOT);
    }

    /// A transclusion `when=` reads the parent's resolved frontmatter, not
    /// its template.
    #[test]
    fn a_transclusion_condition_reads_resolved_frontmatter() {
        let parent = "---\nwho: default\ngreeting: \"Hi {{ who }}\"\n---\n::file ./leaf.md when=\"greeting == 'Hi supplied'\"\n";
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "parent.md", parent);
        write(dir.path(), "leaf.md", "[Leaf](./missing.md)\n");
        let root_text = "::file ./parent.md interpolate=false set.who=\"supplied\"\n";

        let (composed, _) = compose_root(dir.path(), root_text, |options| options).unwrap();
        assert!(composed.content().contains("[Leaf]("), "composition includes the leaf: {}", composed.content());

        let (links, errors) = references(dir.path(), root_text);
        assert_eq!(links, ["./missing.md"], "the graph includes the leaf");
        assert!(errors.len() == 1 && errors[0].contains("missing.md"), "{errors:?}");
    }

    /// A prologue target written as a template resolves from the parent's
    /// resolved frontmatter, as composition reads it.
    #[test]
    fn a_templated_prologue_target_resolves_like_composition() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "parent.md", "---\nwho: default\nprologue: \"./{{ who }}.md\"\n---\nParent.\n");
        write(dir.path(), "supplied.md", "[Leaf](./missing.md)\n");
        let root_text = "::file ./parent.md interpolate=false set.who=\"supplied\"\n";

        let (composed, _) = compose_root(dir.path(), root_text, |options| options).unwrap();
        assert!(composed.content().contains("[Leaf]("), "composition includes the prologue: {}", composed.content());

        let (links, errors) = references(dir.path(), root_text);
        assert_eq!(links, ["./missing.md"], "the graph includes the prologue");
        assert!(errors.len() == 1 && errors[0].contains("missing.md"), "{errors:?}");
    }

    /// Control: the leaf included directly with the value.
    #[test]
    fn a_direct_set_on_the_leaf_records_the_link() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "leaf.md", LEAF);
        let (links, errors) = references(dir.path(), "::file ./leaf.md set.word=\"supplied\"\n");
        assert_eq!(links, ["./missing.md"]);
        assert!(errors.len() == 1 && errors[0].contains("missing.md"), "{errors:?}");
    }

    /// One leaf reached through two inclusions of its parent, with different
    /// inherited values, is prepared under each: each copy's selected link
    /// is recorded.
    #[test]
    fn a_shared_grandchild_is_prepared_under_each_inherited_value() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "parent.md", "::file ./leaf.md\n");
        write(dir.path(), "leaf.md", "[Link](./{{ word }}.md)\n");
        let root_text = "::file ./parent.md set.word=\"one\"\n\n::file ./parent.md set.word=\"two\"\n";
        let (links, _) = references(dir.path(), root_text);
        assert_eq!(links, ["./one.md", "./two.md"]);
    }

    /// A fragment naming the heading of the first of two copies of a shared
    /// grandchild is valid: each copy's prepared headings are kept, not just
    /// the last copy's.
    #[test]
    fn each_copy_of_a_shared_grandchild_keeps_its_prepared_headings() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "parent.md", "::file ./leaf.md\n");
        write(dir.path(), "leaf.md", "## Heading {{ word }}\n");
        let root = dir.path().join("root.md");
        let root_text =
            "[First](#heading-one)\n\n::file ./parent.md set.word=\"one\"\n\n::file ./parent.md set.word=\"two\"\n";

        let (composed, _) = compose_root(dir.path(), root_text, |options| options).unwrap();
        assert!(composed.content().contains("## Heading one"), "{}", composed.content());

        let request =
            crate::markdown::compose::test_request(ComposeOptions::new().with_source_file(&root).for_reference_check());
        let options = ReferenceValidationOptions {
            validate_fragments: true,
            ..ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(&request))
        };
        let report = Markdown::try_from(root.as_path()).unwrap().validate_references(options).unwrap();
        assert!(report.issues.is_empty(), "the first copy's heading is a valid fragment: {:?}", report.issues);
    }

    /// R4-F1: one file included twice with different values is prepared
    /// twice, and every projection of the graph follows the copy its own
    /// inclusion prepared: the composed references, both validation APIs
    /// (and where each issue is attached), the followed graph JSON, and the
    /// file-tree model with its issue labels.
    mod repeated_copies {
        use super::*;
        use crate::markdown::reference::file_tree::model::{FileTreeNode, build_file_tree_model};

        /// Selects `present.md` under `word == 'a'` and `missing.md` under `'b'`.
        const SELECTING_LEAF: &str = "::block when=\"word == 'a'\"\n[Good](./present.md)\n::end-block\n::block when=\"word == 'b'\"\n[Bad](./missing.md)\n::end-block\n";
        const SET_A: &str = " set.word=\"a\"";
        const SET_B: &str = " set.word=\"b\"";

        /// A fixture: the files beside `root.md`, the file `root.md` includes
        /// twice, and the options of the good (`present.md`) and bad
        /// (`missing.md`) inclusion.
        struct Shape {
            files: &'static [(&'static str, &'static str)],
            included: &'static str,
            good: &'static str,
            bad: &'static str,
        }

        /// Every followed projection of `root.md`, each in composed order.
        #[derive(Debug, PartialEq)]
        struct Projections {
            composed: Vec<String>,
            typed_links: Vec<String>,
            /// `{reference}@{line of the root inclusion that reached it}`.
            one_step_errors: Vec<String>,
            prebuilt_errors: Vec<String>,
            view: Vec<String>,
            tree: Vec<String>,
        }

        fn errors(report: &crate::markdown::reference::validate::ReferenceValidationReport) -> Vec<String> {
            report
                .issues
                .iter()
                .filter(|issue| issue.severity == ReferenceSeverity::Error)
                .map(|issue| {
                    let line = issue.included_via.first().and_then(|step| step.line);
                    format!("{}@{}", issue.reference_display, line.map_or("-".to_string(), |line| line.to_string()))
                })
                .collect()
        }

        fn view_links(node: &serde_json::Value, out: &mut Vec<String>) {
            for record in node["references"].as_array().into_iter().flatten() {
                if record["kind"] == "hyperlink" {
                    out.push(record["target"]["raw"].as_str().unwrap_or_default().to_string());
                }
            }
            for insertion in node["transclusions"].as_array().into_iter().flatten() {
                if let Some(child) = insertion.get("node") {
                    view_links(child, out);
                }
            }
        }

        fn tree_rows(node: &FileTreeNode, out: &mut Vec<String>) {
            for row in node.reference_groups.iter().flat_map(|group| &group.rows) {
                match row.validation.as_ref().and_then(|validation| validation.suffix.as_deref()) {
                    Some(suffix) => out.push(format!("{} {suffix}", row.display_target)),
                    None => out.push(row.display_target.clone()),
                }
            }
            for child in &node.children {
                tree_rows(child, out);
            }
        }

        fn projections(root: &Path) -> Projections {
            let request =
                crate::markdown::compose::test_request(ComposeOptions::new().with_source_file(root).for_reference_check());
            let graph_options = ReferenceGraphOptions::with_compose(&request);
            let md = Markdown::try_from(root).unwrap();
            let target = |record: &crate::markdown::reference::ReferenceRecord| record.target.raw().unwrap().to_string();

            let composed =
                md.composed_references(graph_options.clone()).unwrap().hyperlinks().into_iter().map(target).collect();
            let typed_links = md
                .composed_links(graph_options.clone())
                .unwrap()
                .iter()
                .map(|link| target(&link.record))
                .collect();
            let options = ReferenceValidationOptions::with_graph(graph_options.clone());
            let one_step = md.validate_references(options.clone()).unwrap();
            let graph = md.reference_graph(graph_options).unwrap();
            let prebuilt = md.validate_references_with_graph(&graph, options).unwrap();
            let mut view = Vec::new();
            view_links(&serde_json::to_value(graph.view(true)).unwrap(), &mut view);
            let mut tree = Vec::new();
            tree_rows(&build_file_tree_model(&graph, Some(&one_step), true).root, &mut tree);
            Projections {
                composed,
                typed_links,
                one_step_errors: errors(&one_step),
                prebuilt_errors: errors(&prebuilt),
                view,
                tree,
            }
        }

        /// Both inclusion orders under both prose modes: composition emits
        /// both links, and every projection reports the same two in order,
        /// with the one missing target attached to the inclusion that reached it.
        fn case(shape: &Shape) {
            for prose in ["", " interpolate=false"] {
                for good_first in [true, false] {
                    let dir = tempfile::tempdir().unwrap();
                    write(dir.path(), "present.md", "# Present\n");
                    for (name, text) in shape.files {
                        write(dir.path(), name, text);
                    }
                    let (first, second) = if good_first { (shape.good, shape.bad) } else { (shape.bad, shape.good) };
                    let included = shape.included;
                    let root_text = format!("::file ./{included}{prose}{first}\n\n::file ./{included}{prose}{second}\n");

                    let (composed, _) = compose_root(dir.path(), &root_text, |options| options).unwrap();
                    let content = composed.content();
                    assert!(content.contains("present.md)") && content.contains("missing.md)"), "{content}");

                    let links: Vec<String> = if good_first {
                        vec!["./present.md".into(), "./missing.md".into()]
                    } else {
                        vec!["./missing.md".into(), "./present.md".into()]
                    };
                    let error = vec![format!("./missing.md@{}", if good_first { 3 } else { 1 })];
                    let tree = links
                        .iter()
                        .map(|link| if link == "./missing.md" { format!("{link} [missing]") } else { link.clone() })
                        .collect();
                    let expected = Projections {
                        composed: links.clone(),
                        typed_links: links.clone(),
                        one_step_errors: error.clone(),
                        prebuilt_errors: error,
                        view: links,
                        tree,
                    };
                    assert_eq!(
                        projections(&dir.path().join("root.md")),
                        expected,
                        "prose={prose:?} good_first={good_first}"
                    );
                }
            }
        }

        macro_rules! repeated_copy_cases {
            ($($name:ident: $shape:expr;)+) => {$(
                #[test]
                fn $name() {
                    case(&$shape);
                }
            )+};
        }

        repeated_copy_cases! {
            a_direct_body_inclusion: Shape {
                files: &[("leaf.md", SELECTING_LEAF)],
                included: "leaf.md",
                good: SET_A,
                bad: SET_B,
            };
            a_body_inclusion_through_an_intermediate_parent: Shape {
                files: &[("parent.md", "::file ./leaf.md\n"), ("leaf.md", SELECTING_LEAF)],
                included: "parent.md",
                good: SET_A,
                bad: SET_B,
            };
            a_prologue_child: Shape {
                files: &[("parent.md", "---\nprologue: ./leaf.md\n---\nParent.\n"), ("leaf.md", SELECTING_LEAF)],
                included: "parent.md",
                good: SET_A,
                bad: SET_B,
            };
            an_epilogue_child: Shape {
                files: &[("parent.md", "---\nepilogue: ./leaf.md\n---\nParent.\n"), ("leaf.md", SELECTING_LEAF)],
                included: "parent.md",
                good: SET_A,
                bad: SET_B,
            };
            a_resolved_descendant_target: Shape {
                files: &[
                    ("parent.md", "::file ./{{ word }}.md\n"),
                    ("a.md", "[Good](./present.md)\n"),
                    ("b.md", "[Bad](./missing.md)\n"),
                ],
                included: "parent.md",
                good: SET_A,
                bad: SET_B,
            };
            a_replacement_dependent_child: Shape {
                files: &[("leaf.md", "[Link](./WORD.md)\n")],
                included: "leaf.md",
                good: r#" replace='{"WORD":"present"}'"#,
                bad: r#" replace='{"WORD":"missing"}'"#,
            };
        }

        /// R4-F1.15 control: two inclusions with the same values compose the
        /// same copy, which every projection reports once per inclusion.
        #[test]
        fn the_same_values_twice_stay_one_copy() {
            for prose in ["", " interpolate=false"] {
                let dir = tempfile::tempdir().unwrap();
                write(dir.path(), "present.md", "# Present\n");
                write(dir.path(), "leaf.md", SELECTING_LEAF);
                let root_text = format!("::file ./leaf.md{prose}{SET_A}\n\n::file ./leaf.md{prose}{SET_A}\n");
                write(dir.path(), "root.md", &root_text);
                let root = dir.path().join("root.md");
                let twice = vec!["./present.md".to_string(), "./present.md".to_string()];
                assert_eq!(
                    projections(&root),
                    Projections {
                        composed: twice.clone(),
                        typed_links: twice.clone(),
                        one_step_errors: Vec::new(),
                        prebuilt_errors: Vec::new(),
                        view: twice.clone(),
                        tree: twice,
                    },
                    "prose={prose:?}"
                );

                let request = crate::markdown::compose::test_request(
                    ComposeOptions::new().with_source_file(&root).for_reference_check(),
                );
                let graph = Markdown::try_from(root.as_path())
                    .unwrap()
                    .reference_graph(ReferenceGraphOptions::with_compose(&request))
                    .unwrap();
                let insertions = &graph.root().child_insertions;
                assert_eq!(insertions.len(), 2);
                assert_eq!(insertions[0].child_node_id, insertions[1].child_node_id, "identical copies share one node id");
            }
        }
    }

    /// R4-F2: an inclusion repeated by a `::map` / `::filter` body reads its
    /// executable inputs (target, both `set` forms, `replace=`, `when=`) in
    /// the frame of its item during reference preparation, as composition
    /// does. Each case composes `parent.md` over `words: [a, b]`, whose item
    /// `a` reaches `present.md`-style targets and item `b` a missing one.
    mod collection_inputs {
        use super::*;

        const PARENT_FRONTMATTER: &str = "---\n$schema:\n  words: string[](required)\nwords: [a, b]\n---\n";
        const SELECTING_LEAF: &str = "::block when=\"word == 'a'\"\n[Good](./present.md)\n::end-block\n::block when=\"word == 'b'\"\n[Bad](./missing.md)\n::end-block\n";

        /// One input reader: the repeated body line, the files it reaches,
        /// and the item-ordered hyperlinks and missing targets it yields.
        struct Reader {
            body: &'static str,
            files: &'static [(&'static str, &'static str)],
            links: &'static [&'static str],
            /// `{severity} {target}` of each issue. A replacement value
            /// that interpolation inserted is data, so its missing target
            /// warns instead of failing, as outside a collection.
            issues: &'static [&'static str],
        }

        const TARGET: Reader = Reader {
            body: "::file ./{{ word }}.md",
            files: &[("a.md", "[A](./present.md)\n"), ("b.md", "[B](./missing.md)\n")],
            links: &["./present.md", "./missing.md"],
            issues: &["Error ./missing.md"],
        };
        const PROPERTY_SET: Reader = Reader {
            body: "::file ./leaf.md set.word=\"{{ word }}\"",
            files: &[("leaf.md", SELECTING_LEAF)],
            links: &["./present.md", "./missing.md"],
            issues: &["Error ./missing.md"],
        };
        const OBJECT_SET: Reader = Reader {
            body: "::file ./leaf.md set='{\"word\":\"{{ word }}\"}'",
            files: &[("leaf.md", SELECTING_LEAF)],
            links: &["./present.md", "./missing.md"],
            issues: &["Error ./missing.md"],
        };
        const REPLACE: Reader = Reader {
            body: "::file ./leaf.md replace='{\"WORD\":\"{{ word }}\"}'",
            files: &[("leaf.md", "[Link](./WORD-target.md)\n"), ("a-target.md", "# A\n")],
            links: &["./a-target.md", "./b-target.md"],
            issues: &["Warning ./b-target.md"],
        };
        const CONDITION: Reader = Reader {
            body: "::file ./b.md when=\"word == 'b'\"",
            files: &[("b.md", "[B](./missing.md)\n")],
            links: &["./missing.md"],
            issues: &["Error ./missing.md"],
        };

        const MAP: &str = "::map words \"word => word\"";
        const FILTER: &str = "::filter words \"word => true\"";

        /// The composed hyperlinks and both validations' error targets, each
        /// with the authored line of every inclusion on its trail, read
        /// through the public reference APIs.
        #[derive(Debug, PartialEq)]
        struct Observed {
            composed: Vec<String>,
            one_step_errors: Vec<String>,
            prebuilt_errors: Vec<String>,
        }

        fn errors(report: &crate::markdown::reference::validate::ReferenceValidationReport) -> Vec<String> {
            report
                .issues
                .iter()
                .filter(|issue| issue.severity != ReferenceSeverity::Info)
                .map(|issue| {
                    let trail: Vec<String> = issue
                        .included_via
                        .iter()
                        .map(|step| step.line.map_or("-".to_string(), |line| line.to_string()))
                        .collect();
                    format!("{:?} {}@{}", issue.severity, issue.reference_display, trail.join(">"))
                })
                .collect()
        }

        fn observe(root: &Path, request: &crate::markdown::compose::ComposeRequest) -> Observed {
            let graph_options = ReferenceGraphOptions::with_compose(request);
            let md = Markdown::try_from(root).unwrap();
            let composed = md
                .composed_references(graph_options.clone())
                .unwrap()
                .hyperlinks()
                .into_iter()
                .map(|record| record.target.raw().unwrap().to_string())
                .collect();
            let options = ReferenceValidationOptions::with_graph(graph_options.clone());
            let one_step = md.validate_references(options.clone()).unwrap();
            let graph = md.reference_graph(graph_options).unwrap();
            let prebuilt = md.validate_references_with_graph(&graph, options).unwrap();
            Observed { composed, one_step_errors: errors(&one_step), prebuilt_errors: errors(&prebuilt) }
        }

        /// Writes the fixture and returns the root path.
        fn fixture(dir: &Path, collection: &str, body: &str, reader: &Reader, prose: &str) -> std::path::PathBuf {
            let closing = if collection.starts_with("::map") { "::end-map" } else { "::end-filter" };
            write(dir, "present.md", "# Present\n");
            for (name, text) in reader.files {
                write(dir, name, text);
            }
            write(dir, "parent.md", &format!("{PARENT_FRONTMATTER}{collection}\n{body}\n{closing}\n"));
            let root = dir.join("root.md");
            std::fs::write(&root, format!("::file ./parent.md{prose}\n")).unwrap();
            root
        }

        fn case(collection: &str, reader: &Reader, prose: &str) {
            let dir = tempfile::tempdir().unwrap();
            let root = fixture(dir.path(), collection, reader.body, reader, prose);

            // R4-F2.11 control: composition reads each item's value.
            let (composed, _) = compose_root(dir.path(), &format!("::file ./parent.md{prose}\n"), |options| options)
                .unwrap();
            // Root link normalization drops the `./`.
            for link in reader.links {
                let normalized = format!("]({})", link.trim_start_matches("./"));
                assert!(composed.content().contains(&normalized), "{link} in {}", composed.content());
            }

            // The parent's repeated directive is authored on line 7.
            let errors: Vec<String> = reader.issues.iter().map(|issue| format!("{issue}@1>7")).collect();
            let expected = Observed {
                composed: reader.links.iter().map(|link| link.to_string()).collect(),
                one_step_errors: errors.clone(),
                prebuilt_errors: errors,
            };
            for request in [
                crate::markdown::compose::test_request(ComposeOptions::new().with_source_file(&root).for_reference_check()),
                crate::markdown::compose::test_request(ComposeOptions::new().with_source_file(&root)),
            ] {
                assert_eq!(observe(&root, &request), expected, "{collection} {:?} prose={prose:?}", reader.body);
            }
        }

        macro_rules! collection_input_cases {
            ($($name:ident: $collection:expr, $reader:expr;)+) => {$(
                mod $name {
                    use super::*;

                    #[test]
                    fn normal_prose() {
                        case($collection, &$reader, "");
                    }

                    #[test]
                    fn literal_prose() {
                        case($collection, &$reader, " interpolate=false");
                    }
                }
            )+};
        }

        collection_input_cases! {
            map_target: MAP, TARGET;
            filter_target: FILTER, TARGET;
            map_property_set: MAP, PROPERTY_SET;
            filter_property_set: FILTER, PROPERTY_SET;
            map_object_set: MAP, OBJECT_SET;
            filter_object_set: FILTER, OBJECT_SET;
            map_replace: MAP, REPLACE;
            filter_replace: FILTER, REPLACE;
            map_condition: MAP, CONDITION;
            filter_condition: FILTER, CONDITION;
        }

        /// Control for `REPLACE`'s severity: the same interpolated replacement
        /// value outside a collection is data too.
        #[test]
        fn an_interpolated_replacement_value_outside_a_collection_is_data_too() {
            let dir = tempfile::tempdir().unwrap();
            write(dir.path(), "leaf.md", "[Link](./WORD-target.md)\n");
            write(dir.path(), "parent.md", "---\nword: b\n---\n::file ./leaf.md replace='{\"WORD\":\"{{ word }}\"}'\n");
            let root = dir.path().join("root.md");
            std::fs::write(&root, "::file ./parent.md\n").unwrap();
            let request =
                crate::markdown::compose::test_request(ComposeOptions::new().with_source_file(&root).for_reference_check());
            let errors = vec!["Warning ./b-target.md@1>4".to_string()];
            assert_eq!(
                observe(&root, &request),
                Observed { composed: vec!["./b-target.md".into()], one_step_errors: errors.clone(), prebuilt_errors: errors }
            );
        }

        /// A `::block` gate inside a copy reads its item, and one outside every
        /// collection keeps its own selection (R4-F2.12).
        #[test]
        fn block_gates_inside_and_outside_a_copy_select_as_composition_does() {
            for collection in [MAP, FILTER] {
                for prose in ["", " interpolate=false"] {
                    let dir = tempfile::tempdir().unwrap();
                    let closing = if collection == MAP { "::end-map" } else { "::end-filter" };
                    write(
                        dir.path(),
                        "parent.md",
                        &format!(
                            "{PARENT_FRONTMATTER}{collection}\n::block when=\"word == 'b'\"\n[B](./missing.md)\n::end-block\n{closing}\n::block when=\"false\"\n[Never](./never.md)\n::end-block\n"
                        ),
                    );
                    let root = dir.path().join("root.md");
                    std::fs::write(&root, format!("::file ./parent.md{prose}\n")).unwrap();
                    let request = crate::markdown::compose::test_request(
                        ComposeOptions::new().with_source_file(&root).for_reference_check(),
                    );
                    let errors = vec!["Error ./missing.md@1".to_string()];
                    assert_eq!(
                        observe(&root, &request),
                        Observed { composed: vec!["./missing.md".into()], one_step_errors: errors.clone(), prebuilt_errors: errors },
                        "{collection} prose={prose:?}"
                    );
                }
            }
        }

        /// A link a copy repeated keeps its authored position in the file
        /// that holds the collection, under both prose modes.
        #[test]
        fn a_link_in_a_copy_reports_its_authored_line() {
            for prose in ["", " interpolate=false"] {
                let dir = tempfile::tempdir().unwrap();
                write(dir.path(), "parent.md", &format!("{PARENT_FRONTMATTER}{MAP}\n[Item](./missing-{{{{ word }}}}.md)\n::end-map\n"));
                let root = dir.path().join("root.md");
                std::fs::write(&root, format!("::file ./parent.md{prose}\n")).unwrap();
                let request = crate::markdown::compose::test_request(
                    ComposeOptions::new().with_source_file(&root).for_reference_check(),
                );
                let report = Markdown::try_from(root.as_path())
                    .unwrap()
                    .validate_references(ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(&request)))
                    .unwrap();
                let located: Vec<(String, Option<usize>)> = report
                    .issues
                    .iter()
                    .map(|issue| (issue.reference_display.clone(), issue.origin.authored.as_ref().map(|at| at.line)))
                    .collect();
                let expected = if prose.is_empty() {
                    vec![("./missing-a.md".to_string(), Some(7)), ("./missing-b.md".to_string(), Some(7))]
                } else {
                    // Literal prose keeps the template, whose spaces leave it
                    // no link at all.
                    Vec::new()
                };
                assert_eq!(located, expected, "prose={prose:?}");
            }
        }

        /// A `::toc-linking` target a copy reads resolves per item.
        #[test]
        fn a_toc_linking_target_in_a_copy_reads_its_item() {
            for collection in [MAP, FILTER] {
                let dir = tempfile::tempdir().unwrap();
                let closing = if collection == MAP { "::end-map" } else { "::end-filter" };
                write(dir.path(), "a.md", "# A\n\n## Alpha\n");
                write(dir.path(), "b.md", "# B\n\n## Beta\n");
                write(
                    dir.path(),
                    "parent.md",
                    &format!("{PARENT_FRONTMATTER}{collection}\n::toc-linking ./{{{{ word }}}}.md\n{closing}\n"),
                );
                let root = dir.path().join("root.md");
                std::fs::write(&root, "::file ./parent.md interpolate=false\n").unwrap();
                let (composed, _) = compose_root(dir.path(), "::file ./parent.md interpolate=false\n", |options| options)
                    .unwrap();
                assert!(composed.content().contains("#alpha") && composed.content().contains("#beta"), "{}", composed.content());

                let request = crate::markdown::compose::test_request(
                    ComposeOptions::new().with_source_file(&root).for_reference_check(),
                );
                let options = ReferenceValidationOptions {
                    validate_fragments: true,
                    ..ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(&request))
                };
                let md = Markdown::try_from(root.as_path()).unwrap();
                let one_step = md.validate_references(options.clone()).unwrap();
                assert!(one_step.issues.is_empty(), "{collection}: {:?}", one_step.issues);
                let graph = md.reference_graph(options.graph.clone()).unwrap();
                let prebuilt = md.validate_references_with_graph(&graph, options).unwrap();
                assert!(prebuilt.issues.is_empty(), "{collection}: {:?}", prebuilt.issues);
                let toc_targets: Vec<String> = graph
                    .nodes()
                    .iter()
                    .flat_map(|node| node.local_references.by_kind(ReferenceKind::Transclusion))
                    .filter(|record| record.origin.syntax == crate::markdown::reference::ReferenceSyntax::DirectiveTocLinking)
                    .map(|record| record.target.raw().unwrap().to_string())
                    .collect();
                assert_eq!(toc_targets, ["./a.md", "./b.md"], "{collection}");
            }
        }

        /// A heading a copy writes is a valid fragment target, as composition
        /// writes one heading per item.
        #[test]
        fn a_heading_in_a_copy_is_a_fragment_target() {
            let dir = tempfile::tempdir().unwrap();
            write(dir.path(), "parent.md", &format!("{PARENT_FRONTMATTER}{FILTER}\n## Heading {{{{ word }}}}\n::end-filter\n"));
            let root = dir.path().join("root.md");
            std::fs::write(&root, "[B](#heading-b)\n\n::file ./parent.md\n").unwrap();
            let (composed, _) = compose_root(dir.path(), "[B](#heading-b)\n\n::file ./parent.md\n", |options| options).unwrap();
            assert!(composed.content().contains("## Heading b"), "{}", composed.content());
            let request =
                crate::markdown::compose::test_request(ComposeOptions::new().with_source_file(&root).for_reference_check());
            let options = ReferenceValidationOptions {
                validate_fragments: true,
                ..ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(&request))
            };
            let report = Markdown::try_from(root.as_path()).unwrap().validate_references(options).unwrap();
            assert!(report.issues.is_empty(), "{:?}", report.issues);
        }

        /// A selecting filter reaches only its retained items: the rejected
        /// item's missing target is not reported, as composition never
        /// produces it.
        #[test]
        fn a_rejected_item_contributes_no_reference() {
            let dir = tempfile::tempdir().unwrap();
            let root = fixture(dir.path(), "::filter words \"word => word == 'a'\"", TARGET.body, &TARGET, "");
            let request =
                crate::markdown::compose::test_request(ComposeOptions::new().with_source_file(&root).for_reference_check());
            assert_eq!(
                observe(&root, &request),
                Observed { composed: vec!["./present.md".into()], one_step_errors: Vec::new(), prebuilt_errors: Vec::new() }
            );
        }

        /// No reference check runs a `::shell` command a collection body
        /// repeats, under either request: the command would need approval,
        /// and its file would appear.
        #[test]
        fn reference_checks_run_no_repeated_shell_command() {
            let dir = tempfile::tempdir().unwrap();
            let marker = dir.path().join("ran");
            let body = format!("::shell touch {}-{{{{ word }}}}\n{}", marker.display(), TARGET.body);
            for prose in ["", " interpolate=false"] {
                let root = fixture(dir.path(), MAP, &body, &TARGET, prose);
                for request in [
                    crate::markdown::compose::test_request(ComposeOptions::new().with_source_file(&root).for_reference_check()),
                    crate::markdown::compose::test_request(ComposeOptions::new().with_source_file(&root)),
                ] {
                    let observed = observe(&root, &request);
                    assert_eq!(observed.composed, ["./present.md", "./missing.md"], "prose={prose:?}");
                }
                for item in ["a", "b"] {
                    assert!(!dir.path().join(format!("ran-{item}")).exists(), "prose={prose:?}");
                }
            }
        }
    }

    /// R4-F1.13/.20–.24: the copies a `::map` / `::filter` repeats from one
    /// directive line share their reference ids and inclusion trail, so the
    /// file tree must attach each validation label only to the reference the
    /// issue describes. Each leaf reaches `present.*` in one copy and
    /// `missing.*` in the other; the present copy carries no label and no
    /// issue.
    mod same_line_copy_labels {
        use super::*;
        use crate::markdown::reference::file_tree::model::{FileTreeNode, build_file_tree_model};

        /// One leaf shape: the repeated directive's options and the leaf text.
        struct Leaf {
            options: &'static str,
            text: &'static str,
        }

        const REPLACED: &str = " replace='{\"WORD\":\"{{ word }}\"}'";
        const SET: &str = " set.word=\"{{ word }}\"";

        /// The labeled rows and edges of one leaf copy, and its node summary.
        #[derive(Debug, PartialEq)]
        struct Copy {
            targets: Vec<String>,
            labels: Vec<String>,
            issues_count: usize,
            has_errors: bool,
        }

        fn copy(node: &FileTreeNode) -> (String, Copy) {
            let mut targets = Vec::new();
            let mut labels = Vec::new();
            let rows = node
                .reference_groups
                .iter()
                .flat_map(|group| &group.rows)
                .map(|row| (&row.display_target, &row.validation));
            let edges = node.transclusions.iter().map(|edge| (&edge.display_target, &edge.validation));
            for (target, validation) in rows.chain(edges) {
                targets.push(target.clone());
                if let Some(suffix) = validation.as_ref().and_then(|validation| validation.suffix.as_deref()) {
                    labels.push(format!("{target} {suffix}"));
                }
            }
            let word = if targets.iter().any(|target| target.contains("missing")) { "missing" } else { "present" };
            assert!(targets.iter().all(|target| !target.contains(if word == "missing" { "present" } else { "missing" })), "{targets:?}");
            let summary = Copy { targets, labels, issues_count: node.validation.issues_count, has_errors: node.validation.has_errors };
            (word.to_string(), summary)
        }

        fn case(leaf: &Leaf) {
            for collection in ["::map words \"word => word\"", "::filter words \"word => true\""] {
                let closing = if collection.starts_with("::map") { "::end-map" } else { "::end-filter" };
                for words in ["[present, missing]", "[missing, present]"] {
                    for prose in ["", " interpolate=false"] {
                        let dir = tempfile::tempdir().unwrap();
                        for name in ["present.md", "present.png", "present.rs", "present.html", "present.js", "present.css"] {
                            write(dir.path(), name, "# Heading\n");
                        }
                        write(dir.path(), "leaf.md", leaf.text);
                        write(
                            dir.path(),
                            "parent.md",
                            &format!(
                                "---\n$schema:\n  words: string[](required)\nwords: {words}\n---\n{collection}\n::file ./leaf.md{}\n{closing}\n",
                                leaf.options
                            ),
                        );
                        let root = dir.path().join("root.md");
                        std::fs::write(&root, format!("::file ./parent.md{prose}\n")).unwrap();
                        let request = crate::markdown::compose::test_request(
                            ComposeOptions::new().with_source_file(&root).for_reference_check(),
                        );
                        let md = Markdown::try_from(root.as_path()).unwrap();
                        let graph = md.reference_graph(ReferenceGraphOptions::with_compose(&request)).unwrap();
                        let report = md
                            .validate_references_with_graph(
                                &graph,
                                ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(&request)),
                            )
                            .unwrap();
                        let context = format!("{collection} {words} prose={prose:?}");
                        let reported: Vec<&str> = report.issues.iter().map(|issue| issue.reference_display.as_str()).collect();
                        assert_eq!(reported.len(), 1, "{context}: {reported:?}");
                        assert!(reported[0].starts_with("./missing."), "{context}: {reported:?}");
                        let missing_is_error = report.issues[0].severity == ReferenceSeverity::Error;

                        let model = build_file_tree_model(&graph, Some(&report), true);
                        let parent = &model.root.children[0];
                        let mut copies: Vec<(String, Copy)> = parent.children.iter().map(copy).collect();
                        copies.sort_by(|a, b| a.0.cmp(&b.0));
                        assert_eq!(copies.len(), 2, "{context}: {copies:?}");
                        let missing_targets = copies[0].1.targets.clone();
                        let present_targets = copies[1].1.targets.clone();
                        assert_eq!(missing_targets.len(), 1, "{context}: {copies:?}");
                        assert_eq!(
                            copies,
                            vec![
                                (
                                    "missing".to_string(),
                                    Copy {
                                        labels: vec![format!("{} [missing]", missing_targets[0])],
                                        targets: missing_targets,
                                        issues_count: 1,
                                        has_errors: missing_is_error,
                                    }
                                ),
                                ("present".to_string(), Copy { targets: present_targets, labels: Vec::new(), issues_count: 0, has_errors: false }),
                            ],
                            "{context}"
                        );
                    }
                }
            }
        }

        macro_rules! same_line_cases {
            ($($name:ident: $leaf:expr;)+) => {$(
                #[test]
                fn $name() {
                    case(&$leaf);
                }
            )+};
        }

        same_line_cases! {
            a_markdown_hyperlink: Leaf { options: REPLACED, text: "[Link](./WORD.md)\n" };
            a_hyperlink_with_a_fragment: Leaf { options: REPLACED, text: "[Link](./WORD.md#heading)\n" };
            a_markdown_image: Leaf { options: REPLACED, text: "![Image](./WORD.png)\n" };
            an_html_hyperlink: Leaf { options: REPLACED, text: "<a href=\"./WORD.html\">Link</a>\n" };
            an_html_script: Leaf { options: REPLACED, text: "<script src=\"./WORD.js\"></script>\n" };
            an_html_stylesheet: Leaf { options: REPLACED, text: "<link rel=\"stylesheet\" href=\"./WORD.css\">\n" };
            a_file_transclusion: Leaf { options: REPLACED, text: "::file ./WORD.md\n" };
            a_code_transclusion: Leaf { options: REPLACED, text: "::code ./WORD.rs\n" };
            a_prologue: Leaf { options: SET, text: "---\nprologue: ./{{ word }}.md\n---\nLeaf.\n" };
            an_epilogue: Leaf { options: SET, text: "---\nepilogue: ./{{ word }}.md\n---\nLeaf.\n" };
        }
    }
}

/// A directive's `replace=` option (a one-off map, or `true` for parent-wins
/// precedence) reaches its child on every path that prepares the child, and
/// identifies the child's cached result and pre-flight walk.
mod replace_options {
    use super::*;
    use crate::markdown::compose::remote::RemoteReadConfig;
    use crate::markdown::reference::validate::{ReferenceSeverity, ReferenceValidationOptions};
    use crate::markdown::reference::{ReferenceGraphOptions, ReferenceKind};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const ONE: &str = r#" replace='{"WORD":"one"}'"#;
    const TWO: &str = r#" replace='{"WORD":"two"}'"#;
    const PARENT_WINS: &str = " replace=true";

    /// The root frontmatter of the precedence cases: the parent's map, which
    /// only `replace=true` lets win over the leaf's own.
    const PARENT_MAP: &str = "---\nreplace:\n  WORD: parent\n---\n";

    /// A leaf that maps `WORD` itself, prefixed to the precedence leaves.
    const CHILD_MAP: &str = "---\nreplace:\n  WORD: child\n---\n";

    /// `root.md` text including `leaf.md` once per entry of `options`.
    fn root_text(frontmatter: &str, prose: &str, options: &[&str]) -> String {
        let directives: Vec<String> = options.iter().map(|option| format!("::file ./leaf.md{prose}{option}\n")).collect();
        format!("{frontmatter}{}", directives.join("\n"))
    }

    /// The values the composed leaves show, in document order.
    fn composed_values(dir: &Path, root: &str, mode: CacheAccessMode) -> Vec<String> {
        let (composed, _) = compose_root(dir, root, |options| options.with_cache_access_mode(mode)).unwrap();
        composed
            .content()
            .lines()
            .filter_map(|line| line.strip_prefix("Value: "))
            .map(str::to_string)
            .collect()
    }

    /// R2-F3.1–.3: each inclusion composes with its own replacement inputs,
    /// with the composed-child cache on or off, in either directive order.
    fn composition_case(leaf_frontmatter: &str, root_frontmatter: &str, options: [&str; 2], expected: [&str; 2], prose: &str, mode: CacheAccessMode) {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "leaf.md", &format!("{leaf_frontmatter}Value: WORD\n"));
        let root = root_text(root_frontmatter, prose, &options);
        assert_eq!(composed_values(dir.path(), &root, mode), expected, "{options:?}{prose:?} {mode:?}");
    }

    macro_rules! composition_cases {
        ($($name:ident: $prose:literal, $mode:ident;)+) => {$(
            mod $name {
                use super::*;

                #[test]
                fn one_off_maps_one_then_two() {
                    composition_case("", "", [ONE, TWO], ["one", "two"], $prose, CacheAccessMode::$mode);
                }

                #[test]
                fn one_off_maps_two_then_one() {
                    composition_case("", "", [TWO, ONE], ["two", "one"], $prose, CacheAccessMode::$mode);
                }

                #[test]
                fn precedence_default_then_parent_wins() {
                    composition_case(CHILD_MAP, PARENT_MAP, ["", PARENT_WINS], ["child", "parent"], $prose, CacheAccessMode::$mode);
                }

                #[test]
                fn precedence_parent_wins_then_default() {
                    composition_case(CHILD_MAP, PARENT_MAP, [PARENT_WINS, ""], ["parent", "child"], $prose, CacheAccessMode::$mode);
                }
            }
        )+};
    }

    composition_cases! {
        cached_disabled_prose: " interpolate=false", ReadWrite;
        cached_normal_prose: "", ReadWrite;
        uncached_disabled_prose: " interpolate=false", Off;
        uncached_normal_prose: "", Off;
    }

    /// R2-F3.4: the approval walk discovers each inclusion's replaced command
    /// and execution runs exactly those bytes.
    fn preflight_case(leaf_frontmatter: &str, root_frontmatter: &str, options: [&str; 2], expected: [&str; 2]) {
        let dir = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        write(dir.path(), "leaf.md", &format!("{leaf_frontmatter}::shell echo WORD\n"));
        let root = root_text(root_frontmatter, " interpolate=false", &options);
        let mut expected: Vec<String> = expected.iter().map(|word| format!("echo {word}")).collect();
        expected.sort();
        assert_eq!(preflight_approvals(dir.path(), home.path(), &root), expected, "{options:?}");

        let recorded = Arc::new(RecordingApproval(Mutex::new(Vec::new())));
        compose_root(dir.path(), &root, shell_options(home.path(), Arc::clone(&recorded))).unwrap();
        let mut ran = recorded.0.lock().unwrap().clone();
        ran.sort();
        assert_eq!(ran, expected, "{options:?}: execution runs the walked bytes");
    }

    #[test]
    fn preflight_walks_each_one_off_map() {
        preflight_case("", "", [ONE, TWO], ["one", "two"]);
    }

    #[test]
    fn preflight_walks_each_precedence_mode() {
        preflight_case(CHILD_MAP, PARENT_MAP, ["", PARENT_WINS], ["child", "parent"]);
    }

    /// Copies of one repeated `::file` site whose `replace=` maps differ are
    /// distinct sites: each copy is walked with its own map, and execution
    /// accepts each copy against the walk.
    #[test]
    fn repeated_copies_differing_only_in_replace_are_distinct_sites() {
        let dir = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        write(dir.path(), "leaf.md", "::shell echo WORD\n");
        let root_text = "---\n$schema:\n  words: string[](required)\nwords: [\"one\", \"two\"]\n---\n::map words \"item => item\"\n::file ./leaf.md replace='{\"WORD\":\"{{ item }}\"}'\n::end-map\n";
        let root = dir.path().join("root.md");
        std::fs::write(&root, root_text).unwrap();
        let handler = Arc::new(RecordingApproval(Mutex::new(Vec::new())));
        let options = shell_options(home.path(), handler)(ComposeOptions::new().with_source_file(&root));
        let walk = Markdown::try_from(root.as_path())
            .unwrap()
            .compose_preflight(&crate::markdown::compose::test_request(options))
            .expect("preflight succeeds");
        let parameters: Vec<u64> = walk.preflight_graph.edges.iter().map(|edge| edge.parameters).collect();
        assert_eq!(parameters.len(), 2, "one edge per copy: {parameters:?}");
        assert_ne!(parameters[0], parameters[1], "each copy's map is part of its identity");
        let mut walked = walk.approval_set();
        walked.sort();
        assert_eq!(walked, ["echo one", "echo two"]);

        let recorded = Arc::new(RecordingApproval(Mutex::new(Vec::new())));
        compose_root(dir.path(), root_text, shell_options(home.path(), Arc::clone(&recorded))).unwrap();
        let mut executed = recorded.0.lock().unwrap().clone();
        executed.sort();
        assert_eq!(executed, walked);
    }

    /// R2-F3.5/.6: reference preparation records the replaced link target,
    /// and the reference check passes with only those targets present.
    fn reference_case(leaf_frontmatter: &str, root_frontmatter: &str, options: [&str; 2], expected: [&str; 2]) {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "leaf.md", &format!("{leaf_frontmatter}[Link](./WORD.md)\n"));
        for word in expected {
            write(dir.path(), &format!("{word}.md"), "# Target\n");
        }
        let root = dir.path().join("root.md");
        std::fs::write(&root, root_text(root_frontmatter, " interpolate=false", &options)).unwrap();
        let request =
            crate::markdown::compose::test_request(ComposeOptions::new().with_source_file(&root).for_reference_check());
        let md = Markdown::try_from(root.as_path()).unwrap();

        let graph = md.reference_graph(ReferenceGraphOptions::with_compose(&request)).unwrap();
        let mut links: Vec<String> = graph
            .nodes()
            .iter()
            .flat_map(|node| node.local_references.by_kind(ReferenceKind::Hyperlink))
            .map(|record| record.target.raw().unwrap().to_string())
            .collect();
        links.sort();
        let mut wanted: Vec<String> = expected.iter().map(|word| format!("./{word}.md")).collect();
        wanted.sort();
        assert_eq!(links, wanted, "{options:?}");

        let report = md
            .validate_references(ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(&request)))
            .unwrap();
        let errors: Vec<_> = report.issues.iter().filter(|issue| issue.severity == ReferenceSeverity::Error).collect();
        assert!(errors.is_empty(), "{options:?}: {errors:?}");
    }

    #[test]
    fn reference_preparation_applies_each_one_off_map() {
        reference_case("", "", [ONE, TWO], ["one", "two"]);
    }

    #[test]
    fn reference_preparation_applies_each_precedence_mode() {
        reference_case(CHILD_MAP, PARENT_MAP, ["", PARENT_WINS], ["child", "parent"]);
    }

    /// A `::toc-linking` target is read raw by composition, so the reference
    /// check prepares it without the one-off map of the inclusion that holds
    /// the directive: its fragments name the headings composition linked.
    #[test]
    fn a_toc_linking_target_takes_no_one_off_map_from_the_inclusion() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "parent.md", "::toc-linking ./toc.md\n");
        write(dir.path(), "toc.md", "# Toc\n\n## Heading WORD\n");
        let root = dir.path().join("root.md");
        let root_text = format!("::file ./parent.md{ONE}\n");

        let (composed, _) = compose_root(dir.path(), &root_text, |options| options).unwrap();
        assert!(composed.content().contains("#heading-word"), "composition links the raw heading: {}", composed.content());

        let request =
            crate::markdown::compose::test_request(ComposeOptions::new().with_source_file(&root).for_reference_check());
        let options = ReferenceValidationOptions {
            validate_fragments: true,
            ..ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(&request))
        };
        let report = Markdown::try_from(root.as_path()).unwrap().validate_references(options).unwrap();
        assert!(report.issues.is_empty(), "the linked heading is a valid fragment: {:?}", report.issues);
    }

    /// R2-F3.12 control: `::code` applies each inclusion's map, cache on.
    #[test]
    fn code_transclusion_applies_each_one_off_map() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "leaf.txt", "Value: WORD\n");
        let root = format!("::code ./leaf.txt{ONE}\n\n::code ./leaf.txt{TWO}\n");
        assert_eq!(composed_values(dir.path(), &root, CacheAccessMode::ReadWrite), ["one", "two"]);
    }

    async fn serve(body: &str) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/child.md"))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .mount(&server)
            .await;
        server
    }

    fn remote_allowed(options: ComposeOptions) -> ComposeOptions {
        options
            .with_allow_remote_transclusion(true)
            .with_remote_read_config(RemoteReadConfig { allowed_hosts: vec!["127.0.0.1".into()], ..Default::default() })
    }

    /// The approval walk's commands and the commands execution runs for a
    /// root that reaches the loopback `::shell echo WORD` child.
    async fn remote_commands(dir: &Path, root_text: &str) -> (Vec<String>, Vec<String>) {
        let home = tempfile::tempdir().unwrap();
        let root = dir.join("root.md");
        std::fs::write(&root, root_text).unwrap();
        let handler = Arc::new(RecordingApproval(Mutex::new(Vec::new())));
        let request_options = remote_allowed(shell_options(home.path(), handler)(ComposeOptions::new().with_source_file(&root)));
        let walked = Markdown::try_from(root.as_path())
            .unwrap()
            .compose_preflight(&crate::markdown::compose::test_request(request_options))
            .expect("preflight succeeds")
            .approval_set();

        let recorded = Arc::new(RecordingApproval(Mutex::new(Vec::new())));
        compose_root(dir, root_text, |options| remote_allowed(shell_options(home.path(), Arc::clone(&recorded))(options)))
            .unwrap();
        let ran = recorded.0.lock().unwrap().clone();
        (walked, ran)
    }

    /// R2-F3.8–.11: a remote child's commands are approved and run with the
    /// directive's one-off map applied.
    async fn remote_case(keyword: &str) {
        let server = serve("::shell echo WORD\n").await;
        let dir = tempfile::tempdir().unwrap();
        let root_text = format!("{keyword} {}/child.md interpolate=false{ONE}\n", server.uri());
        let (walked, ran) = remote_commands(dir.path(), &root_text).await;
        assert_eq!(walked, ["echo one"], "{keyword}: preflight");
        assert_eq!(ran, ["echo one"], "{keyword}: execution");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_remote_file_child_applies_its_one_off_map() {
        remote_case("::file").await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_remote_url_child_applies_its_one_off_map() {
        remote_case("::url").await;
    }

    /// A remote grandchild included without `replace=` takes no one-off map
    /// from the inclusion of its parent, as a local grandchild does not.
    async fn remote_grandchild_case(keyword: &str) {
        let server = serve("::shell echo WORD\n").await;
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "local.md", "::shell echo WORD\n");
        write(dir.path(), "local-parent.md", "::file ./local.md\n");
        let (walked, ran) = remote_commands(dir.path(), &format!("::file ./local-parent.md{ONE}\n")).await;
        assert_eq!((walked, ran), (vec!["echo WORD".to_string()], vec!["echo WORD".to_string()]), "local control");

        write(dir.path(), "parent.md", &format!("{keyword} {}/child.md\n", server.uri()));
        let (walked, ran) = remote_commands(dir.path(), &format!("::file ./parent.md{ONE}\n")).await;
        assert_eq!(walked, ["echo WORD"], "{keyword}: preflight");
        assert_eq!(ran, ["echo WORD"], "{keyword}: execution");
    }

    /// A remote `prologue:` of a parent included with a one-off map takes no
    /// map, on the approval walk as in execution.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_remote_prologue_takes_no_one_off_map_from_its_parent() {
        let server = serve("::shell echo WORD\n").await;
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "parent.md", &format!("---\nprologue: {}/child.md\n---\nParent.\n", server.uri()));
        let (walked, ran) = remote_commands(dir.path(), &format!("::file ./parent.md{ONE}\n")).await;
        assert_eq!(walked, ["echo WORD"], "preflight");
        assert_eq!(ran, ["echo WORD"], "execution");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_remote_file_grandchild_takes_no_one_off_map_from_its_parent() {
        remote_grandchild_case("::file").await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_remote_url_grandchild_takes_no_one_off_map_from_its_parent() {
        remote_grandchild_case("::url").await;
    }
}

/// `::toc-linking` links a target's raw headings, so fragment validation
/// checks those generated anchors against the same raw headings, while an
/// ordinary link to the same target keeps the prepared headings.
mod toc_fragments {
    use super::*;
    use crate::markdown::reference::ReferenceGraphOptions;
    use crate::markdown::reference::validate::{ReferenceIssueCode, ReferenceValidationOptions};

    /// The target maps its own `WORD`; composition links the raw `#hello-word`.
    const REPLACED: &str = "---\nreplace:\n  WORD: World\n---\n# Toc\n\n## Hello WORD\n";
    /// The target's heading is a template; composition links `#hello-name`.
    const TEMPLATED: &str = "---\nname: World\n---\n# Toc\n\n## Hello {{ name }}\n";
    /// The target's heading sits in a removed block; composition links `#hidden`.
    const BLOCKED: &str = "# Toc\n\n::block when=\"false\"\n## Hidden\n::end-block\n";

    const TOC_HOLDER: &str = "::toc-linking ./toc.md\n";

    /// Writes the fixture and `root.md`, and returns `root.md`'s path after
    /// checking that composition links `anchor`.
    fn fixture(dir: &Path, toc: &str, files: &[(&str, &str)], root_text: &str, anchor: &str) -> std::path::PathBuf {
        write(dir, "toc.md", toc);
        for (name, text) in files {
            write(dir, name, text);
        }
        let (composed, _) = compose_root(dir, root_text, |options| options).unwrap();
        assert!(
            composed.content().contains(&format!("(./toc.md#{anchor})")),
            "composition links the raw heading: {}",
            composed.content()
        );
        dir.join("root.md")
    }

    fn validation_options(root: &Path, validate_fragments: bool) -> ReferenceValidationOptions {
        let request =
            crate::markdown::compose::test_request(ComposeOptions::new().with_source_file(root).for_reference_check());
        ReferenceValidationOptions {
            validate_fragments,
            ..ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(&request))
        }
    }

    /// The missing-fragment references of one-step validation.
    fn missing_fragments(root: &Path, validate_fragments: bool) -> Vec<String> {
        let report = Markdown::try_from(root)
            .unwrap()
            .validate_references(validation_options(root, validate_fragments))
            .unwrap();
        missing(&report)
    }

    /// The missing-fragment references of validation over a freshly built graph.
    fn missing_fragments_with_graph(root: &Path) -> Vec<String> {
        let md = Markdown::try_from(root).unwrap();
        let options = validation_options(root, true);
        let graph = md.reference_graph(options.graph.clone()).unwrap();
        missing(&md.validate_references_with_graph(&graph, options).unwrap())
    }

    fn missing(report: &crate::markdown::reference::validate::ReferenceValidationReport) -> Vec<String> {
        report
            .issues
            .iter()
            .filter(|issue| issue.code == ReferenceIssueCode::MissingFragmentTarget)
            .map(|issue| issue.reference_display.clone())
            .collect()
    }

    fn body_case(toc: &str, root_text: &str, anchor: &str) {
        let dir = tempfile::tempdir().unwrap();
        let root = fixture(dir.path(), toc, &[("parent.md", TOC_HOLDER)], root_text, anchor);
        assert_eq!(missing_fragments(&root, true), Vec::<String>::new(), "the generated #{anchor} is valid");
        assert_eq!(missing_fragments_with_graph(&root), Vec::<String>::new(), "the generated #{anchor} is valid");
    }

    /// R3-F1.4: body inclusion, the target's own replacement, both prose modes.
    #[test]
    fn a_toc_anchor_of_a_replaced_heading_is_valid_in_a_body_inclusion() {
        body_case(REPLACED, "::file ./parent.md interpolate=false\n", "hello-word");
        body_case(REPLACED, "::file ./parent.md\n", "hello-word");
    }

    /// R3-F1.2: a heading template under normal prose.
    #[test]
    fn a_toc_anchor_of_a_templated_heading_is_valid_under_normal_prose() {
        body_case(TEMPLATED, "::file ./parent.md\n", "hello-name");
    }

    /// R3-F1.2: a heading the target's false block removes, under false.
    #[test]
    fn a_toc_anchor_of_a_heading_in_a_false_block_is_valid() {
        body_case(BLOCKED, "::file ./parent.md interpolate=false\n", "hidden");
    }

    /// R3-F1.9 controls: a plain heading, and a template that false leaves raw.
    #[test]
    fn plain_and_suppressed_template_headings_stay_valid() {
        body_case("# Toc\n\n## Plain\n", "::file ./parent.md interpolate=false\n", "plain");
        body_case(TEMPLATED, "::file ./parent.md interpolate=false\n", "hello-name");
    }

    /// R3-F1.5 and .6, with .7: the TOC holder reached as a prologue or an
    /// epilogue under a false ancestor.
    #[test]
    fn a_toc_anchor_is_valid_through_a_prologue_or_epilogue_holder() {
        for key in ["prologue", "epilogue"] {
            let dir = tempfile::tempdir().unwrap();
            let parent = format!("---\n{key}: ./toc_holder.md\n---\nParent.\n");
            let root = fixture(
                dir.path(),
                REPLACED,
                &[("parent.md", &parent), ("toc_holder.md", TOC_HOLDER)],
                "::file ./parent.md interpolate=false\n",
                "hello-word",
            );
            assert_eq!(missing_fragments(&root, true), Vec::<String>::new(), "{key}");
            assert_eq!(missing_fragments_with_graph(&root), Vec::<String>::new(), "{key}");
        }
    }

    /// R3-F1.10: an ordinary link to the same target keeps the prepared
    /// headings, beside the TOC's raw anchor: `#hello-world` is accepted and
    /// the raw `#hello-word` written by hand is not.
    #[test]
    fn an_ordinary_link_keeps_the_prepared_headings() {
        let dir = tempfile::tempdir().unwrap();
        let root_text = "[Prepared](./toc.md#hello-world)\n\n[Raw](./toc.md#hello-word)\n\n::file ./parent.md\n";
        let root = fixture(dir.path(), REPLACED, &[("parent.md", TOC_HOLDER)], root_text, "hello-word");
        assert_eq!(missing_fragments(&root, true), ["./toc.md#hello-word"]);
        assert_eq!(missing_fragments_with_graph(&root), ["./toc.md#hello-word"]);
    }

    /// R3-F1.11: fragment checking stays opt-in.
    #[test]
    fn without_fragment_checking_nothing_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let root_text = "[Raw](./toc.md#hello-word)\n\n::file ./parent.md\n";
        let root = fixture(dir.path(), REPLACED, &[("parent.md", TOC_HOLDER)], root_text, "hello-word");
        assert_eq!(missing_fragments(&root, false), Vec::<String>::new());
    }

    /// Writes `a.md` and `b.md` (each maps its own `WORD`), the other
    /// `files`, and `root.md`, checks that composition links `#hello-word`
    /// in both targets, and returns both validation APIs' missing fragments.
    fn two_target_case(files: &[(&str, &str)], root_text: &str) -> (Vec<String>, Vec<String>) {
        let dir = tempfile::tempdir().unwrap();
        for (name, text) in [("a.md", REPLACED), ("b.md", REPLACED)].iter().chain(files) {
            write(dir.path(), name, text);
        }
        let (composed, _) = compose_root(dir.path(), root_text, |options| options).unwrap();
        for anchor in ["(./a.md#hello-word)", "(./b.md#hello-word)"] {
            assert!(composed.content().contains(anchor), "{root_text}: {}", composed.content());
        }
        let root = dir.path().join("root.md");
        (missing_fragments(&root, true), missing_fragments_with_graph(&root))
    }

    /// R4-F1.8: one holder whose `::toc-linking` target follows an inherited
    /// value, reached twice with different values as a body inclusion and
    /// through a prologue or epilogue: each copy's anchors are checked
    /// against its own target's raw headings, in both orders and prose modes.
    #[test]
    fn a_repeated_holder_checks_each_copy_against_its_own_target() {
        const HOLDER: &str = "::toc-linking ./{{ word }}.md\n";
        let shapes = [
            ("holder.md", ""),
            ("parent.md", "---\nprologue: ./holder.md\n---\nParent.\n"),
            ("parent.md", "---\nepilogue: ./holder.md\n---\nParent.\n"),
        ];
        for (included, parent) in shapes {
            for prose in ["", " interpolate=false"] {
                for (first, second) in [("a", "b"), ("b", "a")] {
                    let root_text = format!(
                        "::file ./{included}{prose} set.word=\"{first}\"\n\n::file ./{included}{prose} set.word=\"{second}\"\n"
                    );
                    let files = [("holder.md", HOLDER), ("parent.md", parent)];
                    assert_eq!(
                        two_target_case(&files, &root_text),
                        (Vec::new(), Vec::new()),
                        "{included} {parent:?} prose={prose:?} first={first}"
                    );
                }
            }
        }
    }

    /// R4-F1.15 controls: separate TOC lines in one holder, and one target
    /// reached by two copies with the same value.
    #[test]
    fn separate_lines_and_repeated_same_targets_stay_valid() {
        let separate = "::toc-linking ./a.md\n\n::toc-linking ./b.md\n";
        assert_eq!(
            two_target_case(&[("holder.md", separate)], "::file ./holder.md\n"),
            (Vec::new(), Vec::new())
        );
        let dir = tempfile::tempdir().unwrap();
        let root = fixture(
            dir.path(),
            REPLACED,
            &[("parent.md", "::toc-linking ./{{ word }}.md\n")],
            "::file ./parent.md set.word=\"toc\"\n\n::file ./parent.md set.word=\"toc\"\n",
            "hello-word",
        );
        assert_eq!(missing_fragments(&root, true), Vec::<String>::new());
        assert_eq!(missing_fragments_with_graph(&root), Vec::<String>::new());
    }
}
