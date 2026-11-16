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
}
