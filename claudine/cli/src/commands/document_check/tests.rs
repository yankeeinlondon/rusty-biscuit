use super::*;

/// The check's own codes are declared in Claudine's catalog, never beside it.
#[test]
fn check_codes_are_cataloged_unique_and_disjoint_from_darkmatter_findings() {
    const CHECK_CODES: &[&str] = &[
        SELECTION_KEY_AS_DATA,
        NO_EXECUTABLE,
        MULTIPLE_EXECUTABLES,
        SEQUENCE_SHAPE,
        PROVIDER_UNRESOLVED,
        PROVIDER_UNKNOWN,
        PROVIDER_NOT_INSTALLED,
        MODEL_REQUIRED,
        INVALID_ARGUMENTS,
        FILE_CHOICE_PENDING,
        INTERNAL_BUG,
    ];
    for code in CHECK_CODES {
        assert!(claudine::diagnostics::code_spec(code).is_some(), "`{code}` is not in Claudine's catalog");
    }
    let mut codes = CHECK_CODES.to_vec();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), CHECK_CODES.len());
    let mut darkmatter_codes =
        darkmatter::findings::FINDING_CODES.iter().chain(darkmatter::markdown::compose::policy::COMPOSE_DIAGNOSTIC_CODES);
    assert!(darkmatter_codes.all(|(held, _)| !CHECK_CODES.contains(held)));
}

#[test]
fn the_summary_line_counts_findings_checks_and_previews() {
    let mut sink = FindingSink::new();
    let subject = sink.register_subject(Subject::document("doc.md", None));
    sink.push(subject, Diagnostic::actionable("x.error", Severity::Error, "bad", "fix it"));
    sink.not_checked(subject, "provider", "no executable", None);
    let outcome = Outcome { file: None, report: sink.finish(), previews: vec!["text".into()], tasks: 2 };
    assert_eq!(
        outcome.summary_line(),
        "Dry run found 1 error and 0 warnings; 1 check not run. Previewed 1 of 2 tasks."
    );
    assert_eq!(outcome.envelope()["outcome"], "failure");
    assert_eq!(outcome.envelope()["file"], Value::Null);
}

/// Rule 10's cohort rule: a missing input warns only when the run could ask
/// for it and every missing shape can be asked for.
#[test]
fn a_missing_input_warns_only_when_the_whole_cohort_can_be_collected() {
    use super::tasks::missing_input_severity;
    assert_eq!(missing_input_severity(true, true), Severity::Warning);
    assert_eq!(missing_input_severity(true, false), Severity::Error);
    assert_eq!(missing_input_severity(false, true), Severity::Error);
    assert_eq!(missing_input_severity(false, false), Severity::Error);
}

#[test]
fn property_segments_follow_keys_and_indexes() {
    use super::tasks::property_segments;
    use biscuit_file::yaml::YamlPathSegment::{Index, Key};
    assert_eq!(
        property_segments("sequence[0].group.tasks[12]"),
        [Key("sequence".into()), Index(0), Key("group".into()), Key("tasks".into()), Index(12)]
    );
    assert_eq!(property_segments("groups[1].tasks[0]"), [Key("groups".into()), Index(1), Key("tasks".into()), Index(0)]);
    assert_eq!(property_segments("[3]"), [Index(3)], "a bare list's item has no key");
    assert_eq!(property_segments("tasks[0][1]"), [Key("tasks".into()), Index(0), Index(1)]);
    assert!(property_segments("").is_empty());
}

#[derive(Debug, clap::Parser)]
struct Probe {
    #[command(flatten)]
    shared: SharedComposeArgs,
}

/// An error Darkmatter can only call `dm.compose.failure` takes Claudine's
/// code, and a file-reference failure keeps the class a real run prints as
/// its `failure:` row.
#[test]
fn an_unclassified_error_takes_claudines_code_and_its_failure_class() {
    use clap::Parser;
    let shared = Probe::try_parse_from(["probe", "--dry-run"]).unwrap().shared;
    let mut check = Check::new(Mode::Compose, shared, false);
    let subject = check.sink.register_subject(Subject::document("doc.md", None));
    let source = biscuit_file::FileReferenceError::RelativeTreeEscape {
        base_dir: PathBuf::from("/tree"),
        candidate: PathBuf::from("/outside.md"),
        reference: "../outside.md".into(),
    };
    let error = CompositionError::InvalidReference { reference: "../outside.md".into(), source };

    check.push_error(subject, &error, Trail::default());

    let report = check.sink.finish();
    let [diagnostic] = report.diagnostics().try_into().expect("one finding");
    assert_eq!(diagnostic.code, "composition.invalid_file_reference");
    let failure = diagnostic.reference_details.as_ref().and_then(|details| details.failure.as_deref());
    assert_eq!(failure, Some("invalid-reference"));
}

/// R1-F10.4: a display wrapper changes how an error is shown, not what it
/// is. Through `WithFrontmatter`, the early-emitted lifecycle wrapper, and
/// `CheckFailed`, a reference keeps its failure class and a sequence error
/// its proven position.
#[test]
fn a_display_wrapper_keeps_the_class_and_site_of_the_error_it_presents() {
    use clap::Parser;
    type Wrap = fn(CompositionError) -> CompositionError;
    let wrappers: [(&str, Wrap); 3] = [
        ("WithFrontmatter", |inner| CompositionError::WithFrontmatter {
            inner: Box::new(inner),
            excerpt: claudine::composition::FrontmatterExcerpt::capture_line("---\na: 1\n---\n", 2, false).unwrap(),
        }),
        ("already emitted", CompositionError::already_emitted),
        ("CheckFailed", |inner| CompositionError::check_failed(inner, FindingSink::new().finish())),
    ];
    for (label, wrap) in wrappers {
        let shared = Probe::try_parse_from(["probe", "--dry-run"]).unwrap().shared;
        let mut check = Check::new(Mode::Compose, shared, false);
        let subject = check.sink.register_subject(Subject::document("doc.md", None));
        let source = biscuit_file::FileReferenceError::RelativeTreeEscape {
            base_dir: PathBuf::from("/tree"),
            candidate: PathBuf::from("/outside.md"),
            reference: "../outside.md".into(),
        };
        let reference = CompositionError::InvalidReference { reference: "../outside.md".into(), source };
        check.push_error(subject, &wrap(reference), Trail::default());
        let parse = CompositionError::SequenceReferenceInvalid {
            context: "`prompt:` in step 1".into(),
            path: PathBuf::from("seq.yaml"),
            problem: "must be a non-empty file reference, got number".into(),
            site: Box::new(claudine::composition::SequenceSite::file(Path::new("seq.yaml")).at_position(7, 13)),
        };
        check.push_error(subject, &wrap(parse), Trail::default());

        let report = check.sink.finish();
        let [reference, parse] = report.diagnostics().try_into().unwrap_or_else(|_| panic!("{label}: two findings"));
        assert_eq!(reference.code, "composition.invalid_file_reference", "{label}");
        let failure = reference.reference_details.as_ref().and_then(|details| details.failure.as_deref());
        assert_eq!(failure, Some("invalid-reference"), "{label}");
        let at = parse.source.as_ref().map(|source| (source.file.clone(), source.line, source.column));
        assert_eq!(at, Some((Some(PathBuf::from("seq.yaml")), Some(7), Some(13))), "{label}");
    }
}

/// R1-F10.1: a Markdown sequence file whose frontmatter does not parse is
/// located in the file's frame (its frontmatter starts on line 2). Lines
/// added above the mistake move it by as many lines; the fixed file loads.
#[test]
fn a_markdown_frontmatter_failure_is_located_in_the_files_frame() {
    use clap::Parser;
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("child.md");
    let located = |text: &str| {
        std::fs::write(&path, text).unwrap();
        let source = darkmatter::markdown::Markdown::try_from(path.as_path()).err()?;
        let error = CompositionError::MarkdownLoad {
            path: path.clone(),
            source: claudine::composition::MarkdownLoadCause::Parse(Box::new(source)),
        };
        let shared = Probe::try_parse_from(["probe", "--dry-run"]).unwrap().shared;
        let check = Check::new(Mode::Sequence, shared, false);
        let source = check.error_source(&error)?;
        Some((source.file?, source.line?, source.column?))
    };
    for n in [0, 3] {
        let text = format!("---\n{}kind: sequence\nsequence:\n  - name: a\n    name: b\n---\n", "# note\n".repeat(n));
        assert_eq!(located(&text), Some((path.clone(), 4 + n, 5)), "+{n}");
    }
    assert_eq!(located("---\nkind: sequence\nsequence:\n  - name: a\n---\n"), None, "the fixed file loads");
}

/// An error neither Darkmatter nor Claudine types is `internal.bug`, never
/// Darkmatter's catch-all.
#[test]
fn an_error_no_crate_types_is_an_internal_bug() {
    use clap::Parser;
    let shared = Probe::try_parse_from(["probe", "--dry-run"]).unwrap().shared;
    let mut check = Check::new(Mode::Compose, shared, false);
    let subject = check.sink.register_subject(Subject::document("doc.md", None));
    let error = std::fmt::Error;

    check.push_error(subject, &error, Trail::default());

    let report = check.sink.finish();
    let [diagnostic] = report.diagnostics().try_into().expect("one finding");
    assert_eq!(diagnostic.code, INTERNAL_BUG);
}

/// A missing input's row follows the real run's collection policy, read from
/// the terminal state the check observed (injected here, so no terminal is
/// opened): a warning only where an interactive run would ask for every
/// missing input, an error where the run is redirected, `--silent`, has
/// prompting disabled, or meets a shape it cannot ask for.
#[test]
fn a_missing_input_warns_only_where_the_real_run_would_ask_for_it() {
    use claudine::composition::{InteractiveShape, MissingProperty, TextFormat};
    use clap::Parser;
    let text = Some(InteractiveShape::Text { format: TextFormat::Plain, min_len: None, max_len: None });
    // (stdin, stderr, `--silent`, `prompt_for_missing`, shape, expected)
    let cases = [
        ("askable on a terminal", true, true, false, true, text.clone(), Severity::Warning),
        ("stdin redirected", false, true, false, true, text.clone(), Severity::Error),
        ("stderr redirected", true, false, false, true, text.clone(), Severity::Error),
        ("--silent", true, true, true, true, text.clone(), Severity::Error),
        ("prompting disabled", true, true, false, false, text.clone(), Severity::Error),
        ("shape it cannot ask for", true, true, false, true, None, Severity::Error),
    ];
    for (label, stdin, stderr, silent, prompt_for_missing, shape, expected) in cases {
        let mut argv = vec!["probe", "--dry-run"];
        if silent {
            argv.push("--silent");
        }
        let shared = Probe::try_parse_from(argv).unwrap().shared;
        let mut check = Check::new(Mode::Compose, shared, stderr);
        check.stdin_is_tty = stdin;
        check.config = Some(claudine::config::claudine_config::ClaudineConfig { prompt_for_missing, ..Default::default() });
        let subject = check.sink.register_subject(Subject::document("doc.md", None));
        let property = MissingProperty {
            name: "topic".into(),
            type_label: Some("string".into()),
            description: None,
            interactive_shape: shape,
        };
        check.push_missing_rows(vec![inputs::MissingRow {
            subject,
            trail: Trail::default(),
            property,
            declared_in: PathBuf::from("doc.md"),
            hint: "pass `topic=<value>` after the file".into(),
        }]);

        let report = check.sink.finish();
        let [diagnostic] = report.diagnostics().try_into().expect("one finding");
        assert_eq!(diagnostic.code, tasks::MISSING_INPUT, "{label}");
        assert_eq!(diagnostic.severity, expected, "{label}");
    }
}
