use super::*;
use crate::commands::wrap::profile::{ConfiguredModel, ModelSource};

/// A failed exit whose stderr (not yet shown to the user) is `stderr`.
fn stderr_exit(exit_code: i32, stderr: &str) -> NativeExit {
    NativeExit::new(exit_code, ProcessTermination::Completed).with_stderr(stderr, false)
}

fn cause(exit: &NativeExit) -> Option<NativeCliCause> {
    classify_native_exit(exit).map(|classified| classified.cause)
}

fn tail(implicit: &[&str]) -> ProviderTail {
    ProviderTail::new(implicit.iter().map(|arg| arg.to_string()).collect(), None)
}

fn report(provider: Provider, exit: &NativeExit, tail: &ProviderTail) -> NativeExitReport {
    AgentErrorReport::for_native_exit(provider, exit, tail, None)
}

// ── Classification: precedence and signatures ──

#[test]
fn termination_decides_before_any_text() {
    let rejection = "error: unexpected argument '--x' found";
    for (termination, expected) in [
        (ProcessTermination::Interrupted, NativeCliCause::Interrupted),
        (ProcessTermination::TimedOut, NativeCliCause::TimedOut),
        (ProcessTermination::LaunchFailed, NativeCliCause::MissingBinary),
    ] {
        let exit = NativeExit::new(1, termination).with_stderr(rejection, false);
        assert_eq!(cause(&exit), Some(expected), "{termination:?}");
    }
    assert_eq!(cause(&stderr_exit(130, rejection)), Some(NativeCliCause::Interrupted));
    assert_eq!(cause(&stderr_exit(143, rejection)), Some(NativeCliCause::Interrupted));
}

#[test]
fn a_successful_exit_or_missing_evidence_is_unclassified() {
    let rejection = "error: unexpected argument '--x' found";
    assert_eq!(cause(&stderr_exit(0, rejection)), None);
    assert_eq!(cause(&NativeExit::new(2, ProcessTermination::Completed)), None);
}

/// Each signature with a positive fixture and a near miss that must not take
/// it, in precedence order.
#[test]
fn every_signature_has_a_positive_and_a_near_miss() {
    let rows: &[(&str, i32, Option<NativeCliCause>)] = &[
        // Missing binary needs exit 127 as well as the text.
        ("sh: codex: command not found", 127, Some(NativeCliCause::MissingBinary)),
        ("error: file not found: plan.md", 1, None),
        // Authentication / permission.
        ("Error: authentication failed: invalid api key", 1, Some(NativeCliCause::AuthOrPermission)),
        ("error: permission denied (os error 13)", 1, Some(NativeCliCause::AuthOrPermission)),
        ("401 Unauthorized", 1, Some(NativeCliCause::AuthOrPermission)),
        (
            "error: unexpected argument '--authentication-mode' found",
            2,
            Some(NativeCliCause::ArgumentRejected { switch: Some("--authentication-mode".into()) }),
        ),
        // API failure (case-sensitive provider prefix).
        ("API Error: 529 overloaded", 1, Some(NativeCliCause::ApiFailure)),
        ("note: no api error: retrying", 1, None),
        // Model not found.
        ("ProviderModelNotFoundError: x", 1, Some(NativeCliCause::ModelNotFound { suggestions: None })),
        ("warning: using the default model", 1, None),
        // Argument rejection.
        (
            "error: unrecognized arguments: --nope",
            2,
            Some(NativeCliCause::ArgumentRejected { switch: Some("--nope".into()) }),
        ),
        ("error: unknown flag: --zz", 2, Some(NativeCliCause::ArgumentRejected { switch: Some("--zz".into()) })),
        (
            "error: unknown option '-q'",
            2,
            Some(NativeCliCause::ArgumentRejected { switch: Some("-q".into()) }),
        ),
        ("error: unexpected argument found", 2, Some(NativeCliCause::ArgumentRejected { switch: None })),
        // The broad phrase is not a signature: auth and API messages use it.
        ("invalid argument: api key", 1, None),
        ("error: invalid argument for the request", 1, None),
        // Missing argument.
        ("error: missing required argument <PROMPT>", 2, Some(NativeCliCause::MissingArgument)),
        (
            "error: the following required arguments were not provided:",
            2,
            Some(NativeCliCause::MissingArgument),
        ),
        ("note: an optional argument was ignored", 2, None),
        ("some unrelated crash output", 1, None),
    ];
    for (text, code, expected) in rows {
        assert_eq!(cause(&stderr_exit(*code, text)), *expected, "{text:?}");
    }
}

#[test]
fn auth_outranks_argument_rejection_and_api_outranks_model() {
    let exit = stderr_exit(2, "error: unknown option '--x'\nError: authentication failed");
    assert_eq!(cause(&exit), Some(NativeCliCause::AuthOrPermission));
    let exit = stderr_exit(1, "model not found: m\nAPI Error: 500");
    assert_eq!(cause(&exit), Some(NativeCliCause::ApiFailure));
}

#[test]
fn stdout_is_read_when_stderr_has_no_signature() {
    let exit = NativeExit::new(2, ProcessTermination::Completed)
        .with_stderr("starting up", false)
        .with_stdout("Usage error: unknown option '--zz'", false);
    let classified = classify_native_exit(&exit).unwrap();
    assert_eq!(classified.cause, NativeCliCause::ArgumentRejected { switch: Some("--zz".into()) });
    assert_eq!(classified.line.as_deref(), Some("Usage error: unknown option '--zz'"));
}

#[test]
fn the_switch_comes_from_the_matched_line_only() {
    let exit = stderr_exit(2, "warning: --legacy is deprecated\nerror: unexpected argument found");
    assert_eq!(cause(&exit), Some(NativeCliCause::ArgumentRejected { switch: None }));
}

#[test]
fn evidence_is_bounded_to_the_last_lines() {
    let long: String = (0..50).map(|i| format!("line {i}\n")).collect();
    let exit = stderr_exit(1, &long);
    let kept = exit.stderr.as_ref().unwrap().text.lines().count();
    assert_eq!(kept, claudine::signals::EXIT_STDERR_TAIL_LINES);
    assert!(format!("{exit:?}").contains("bytes"));
    assert!(!format!("{exit:?}").contains("line 49"));
}

// ── The one report builder ──

#[test]
fn a_rejection_of_a_forwarded_switch_is_correlated_once() {
    let exit = stderr_exit(2, "error: unexpected argument '--badflag' found");
    let built = report(Provider::Codex, &exit, &tail(&["--badflag", "x"]));
    assert!(built.correlated);
    let summary = &built.report.summary;
    assert!(summary.contains("Codex rejected its arguments"), "{summary}");
    assert!(summary.contains("likely caused by the forwarded arguments: --badflag."), "{summary}");
    assert!(!summary.contains("recogni"), "{summary}");
    assert_eq!(
        built.report.detail.as_deref(),
        Some("error: unexpected argument '--badflag' found")
    );
    assert_eq!(built.report.exit_code, 2);
}

#[test]
fn a_rejection_naming_an_injected_switch_stays_generic() {
    let exit = stderr_exit(2, "error: unexpected argument '--output-last-message' found");
    let built = report(Provider::Codex, &exit, &tail(&["-c", "x=y"]));
    assert!(!built.correlated);
    assert!(built.report.summary.contains("did not recognize a flag"));
    assert!(!built.report.summary.contains("likely caused"));
}

#[test]
fn an_unnamed_rejection_with_an_operand_only_explicit_tail_is_correlated() {
    let explicit = ProviderTail::new(Vec::new(), Some(vec!["stray".into()]));
    let exit = stderr_exit(2, "error: unexpected argument found");
    let built = report(Provider::Codex, &exit, &explicit);
    assert!(built.correlated);
    assert!(
        built.report.summary.contains("an opaque argument tail (passed after --)"),
        "{}",
        built.report.summary
    );
}

#[test]
fn short_attached_and_equals_forms_belong_to_the_tail() {
    let forwarded = tail(&["-cmodel=x", "--token=abc"]);
    assert!(tail_names_switch(&forwarded, "-c"));
    assert!(tail_names_switch(&forwarded, "--token"));
    assert!(tail_names_switch(&forwarded, "--token=abc"));
    assert!(!tail_names_switch(&forwarded, "--tok"));
    assert!(!tail_names_switch(&forwarded, "-m"));
}

#[test]
fn no_other_cause_or_empty_tail_is_misattributed() {
    let forwarded = tail(&["--badflag"]);
    let rejection = "error: unexpected argument '--badflag' found";
    for exit in [
        NativeExit::new(1, ProcessTermination::TimedOut).with_stderr(rejection, false),
        NativeExit::new(130, ProcessTermination::Interrupted).with_stderr(rejection, false),
        stderr_exit(1, "Error: authentication failed"),
        stderr_exit(1, "API Error: 500 internal"),
        stderr_exit(1, "ambiguous: something odd happened"),
    ] {
        let built = report(Provider::Codex, &exit, &forwarded);
        assert!(!built.correlated, "{exit:?}: {}", built.report.summary);
        assert!(!built.report.summary.contains("likely caused"));
    }
    let built = report(Provider::Codex, &stderr_exit(2, rejection), &ProviderTail::default());
    assert!(!built.correlated);
}

#[test]
fn excerpts_mask_tail_secrets_and_escape_the_terminal() {
    let forwarded = tail(&["--password", "hunter2222", "--api-key", "sk-proj-abcdef0123456789"]);
    let exit = stderr_exit(
        2,
        "error: unexpected argument 'hunter2222' found \u{1b}[31m<red>sk-proj-abcdef0123456789</red>",
    );
    let built = report(Provider::Codex, &exit, &forwarded);
    assert!(built.correlated);
    let detail = built.report.detail.unwrap();
    assert!(!detail.contains("hunter2222"), "{detail}");
    assert!(!detail.contains("sk-proj-abcdef0123456789"), "{detail}");
    assert!(!detail.contains('\u{1b}'), "{detail}");
    assert!(!detail.contains("<red>"), "markup must be escaped: {detail}");
}

#[test]
fn a_line_the_user_already_saw_is_not_repeated() {
    let exit = NativeExit::new(2, ProcessTermination::Completed)
        .with_stderr("error: unexpected argument '--badflag' found", true);
    let built = report(Provider::Codex, &exit, &tail(&["--badflag"]));
    assert!(built.correlated);
    assert_eq!(built.report.detail, None);
    assert!(built.report.hint.unwrap().contains("shown above"));

    // The composition path's failure headline already showed the line.
    let headlined = stderr_exit(2, "error: unexpected argument '--badflag' found")
        .with_shown_headline("error: unexpected argument '--badflag' found (attempt 2)");
    let built = report(Provider::Codex, &headlined, &tail(&["--badflag"]));
    assert!(built.correlated);
    assert_eq!(built.report.detail, None);

    let generic = report(Provider::Codex, &NativeExit::new(1, ProcessTermination::Completed)
        .with_stderr("boom", true), &ProviderTail::default());
    assert_eq!(generic.report.detail, None);
}

#[test]
fn unclassified_exits_keep_the_generic_report() {
    let built = report(Provider::Claude, &stderr_exit(3, "\nfirst real line\nsecond"), &tail(&["-x"]));
    assert!(!built.correlated);
    assert_eq!(built.report.category, AgentErrorCategory::AgentNative);
    assert_eq!(built.report.summary, "Claude exited with error code 3");
    assert_eq!(built.report.detail.as_deref(), Some("first real line"));
}

#[test]
fn categories_and_remediation_survive_for_each_cause() {
    let none = ProviderTail::default();
    let auth = report(Provider::Codex, &stderr_exit(1, "Error: authentication failed"), &none).report;
    assert_eq!(auth.category, AgentErrorCategory::Configuration);
    assert!(auth.hint.unwrap().contains("API keys"));

    let api = report(Provider::Codex, &stderr_exit(1, "API Error: 500"), &none).report;
    assert_eq!(api.category, AgentErrorCategory::ApiRemote);
    assert_eq!(api.detail.as_deref(), Some("API Error: 500"));

    let interrupted = report(Provider::OpenCode, &NativeExit::new(130, ProcessTermination::Completed), &none);
    assert_eq!(interrupted.report.category, AgentErrorCategory::Interrupted);

    let timed_out = report(Provider::OpenCode, &NativeExit::new(1, ProcessTermination::TimedOut), &none);
    assert!(timed_out.report.summary.contains("timeout"));
}

// ── Model reports ──

fn opencode_report(stderr: &str, model_source: Option<&ModelSource>) -> AgentErrorReport {
    AgentErrorReport::for_native_exit(
        Provider::OpenCode,
        &stderr_exit(1, stderr),
        &ProviderTail::default(),
        model_source,
    )
    .report
}

#[test]
fn classify_provider_model_not_found_error() {
    let stderr = "Error: ProviderModelNotFoundError: model xyz not found\nsuggestions: [\"abc/one\", \"abc/two\"]";
    let source = ModelSource::CliSwitch("xyz".to_string());
    let report = opencode_report(stderr, Some(&source));
    assert_eq!(report.category, AgentErrorCategory::AgentNative);
    assert!(report.summary.contains("Invalid model specified in the --model CLI switch"));
    assert_eq!(report.suggestions, Some(vec!["abc/one".to_string(), "abc/two".to_string()]));
    assert_eq!(report.location.as_deref(), Some("the --model CLI switch"));
}

#[test]
fn classify_model_not_found_from_env_and_config_sources() {
    let source = ModelSource::ProviderEnv {
        var: "OPENCODE_MODEL",
        model: "bad".to_string(),
    };
    let report = opencode_report("model not found: invalid model name", Some(&source));
    assert!(report.summary.contains("the OPENCODE_MODEL environment variable"));

    let source = ModelSource::ConfigDefault(ConfiguredModel {
        model: "bad".to_string(),
        path: std::path::PathBuf::from("/home/u/.config/opencode/opencode.jsonc"),
    });
    let report = opencode_report("invalid model specified", Some(&source));
    assert_eq!(
        report.location.as_deref(),
        Some("the config file /home/u/.config/opencode/opencode.jsonc")
    );
}

#[test]
fn model_not_found_without_source_uses_default_location() {
    let report = opencode_report("ProviderModelNotFoundError: nope\nsuggestions: [\"a\"]", None);
    assert!(report.summary.contains("the command line"));
    assert_eq!(report.suggestions.as_ref().unwrap()[0], "a");
    let report = opencode_report("ProviderModelNotFoundError: model xyz not found", None);
    assert!(report.suggestions.is_none());
    assert_eq!(report.location, None);
}

#[test]
fn suggestions_parse_from_the_payload() {
    assert_eq!(
        parse_model_suggestions("x\nSuggestions: [\"provider/a\", \"provider/b\"]\nmore"),
        Some(vec!["provider/a".to_string(), "provider/b".to_string()])
    );
    assert_eq!(parse_model_suggestions("no suggestions here"), None);
    assert_eq!(parse_model_suggestions("suggestions: []"), None);
}

// ── Fixed reports ──

#[test]
fn no_model_provided_report_has_expected_content() {
    let report = AgentErrorReport::no_model_provided(Provider::OpenCode);
    assert_eq!(report.exit_code, 1);
    assert_eq!(report.category, AgentErrorCategory::Configuration);
    assert!(report.summary.contains("No model specified"));
    assert!(report.summary.contains("<blue>~/.config/opencode/opencode.json</blue>"));
    let body_list = report.body_list.as_ref().unwrap();
    assert!(body_list.iter().any(|s| s.contains("OPENCODE_MODEL")));
    assert!(body_list.iter().any(|s| s.contains("--model")));
    assert!(report.footer.unwrap().contains("<yellow>opencode models</yellow>"));
    assert_eq!(report.suggestion_style, SuggestionStyle::BareList);
}

#[test]
fn invalid_model_report_has_suggestions_and_location() {
    let report = AgentErrorReport::invalid_model(
        Provider::OpenCode,
        1,
        "the --model CLI switch".to_string(),
        vec!["suggestion/a".to_string(), "suggestion/b".to_string()],
    );
    assert!(report.summary.contains("Invalid model specified in the --model CLI switch"));
    assert_eq!(report.suggestions.as_ref().unwrap().len(), 2);
    assert_eq!(report.location.as_deref(), Some("the --model CLI switch"));
}

#[test]
fn semantic_error_kind_maps_to_agent_error_category() {
    for (kind, category) in [
        (SemanticErrorKind::Configuration, AgentErrorCategory::Configuration),
        (SemanticErrorKind::AgentNative, AgentErrorCategory::AgentNative),
        (SemanticErrorKind::ApiRemote, AgentErrorCategory::ApiRemote),
        (SemanticErrorKind::Interrupted, AgentErrorCategory::Interrupted),
        (SemanticErrorKind::Unknown, AgentErrorCategory::AgentNative),
    ] {
        assert_eq!(AgentErrorCategory::from(kind), category);
    }
}

// ── Display rows ──

fn report_rows(report: &AgentErrorReport) -> Vec<String> {
    let term = Terminal::builder()
        .width(300)
        .color_depth(biscuit_terminal::discovery::detection::ColorDepth::None)
        .build();
    biscuit_terminal::utils::escape_codes::strip_escape_codes(report.status_block(&term).render(&term))
        .lines()
        .map(|line| line.trim_start().trim_start_matches('┃').trim().to_string())
        .filter(|row| !row.is_empty())
        .collect()
}

#[test]
fn every_report_part_starts_on_a_row_of_its_own() {
    let report = AgentErrorReport {
        provider: Provider::OpenCode,
        exit_code: 1,
        category: AgentErrorCategory::AgentNative,
        summary: "summary one\nsummary two".into(),
        body_list: Some(vec!["item one".into(), "item two".into()]),
        footer: Some("footer".into()),
        detail: Some("detail".into()),
        hint: Some("hint".into()),
        suggestions: None,
        suggestion_style: SuggestionStyle::BareList,
        location: None,
    };
    let rows = report_rows(&report);
    let start = rows
        .iter()
        .position(|row| row.starts_with("Agent Error (OpenCode, exit 1)"))
        .unwrap_or_else(|| panic!("heading row: {rows:#?}"));
    let expected = [
        "summary one",
        "summary two",
        "- item one",
        "- item two",
        "footer",
        "detail",
        "hint",
    ];
    assert_eq!(rows[start + 1..start + 1 + expected.len()], expected, "{rows:#?}");
}

#[test]
fn the_no_model_report_keeps_its_authored_rows() {
    let report = AgentErrorReport::no_model_provided(Provider::OpenCode);
    let rows = report_rows(&report);
    for authored in report.summary.lines().chain(report.footer.as_deref().unwrap().lines()) {
        let plain = authored
            .replace("<yellow>", "")
            .replace("</yellow>", "")
            .replace("<blue>", "")
            .replace("</blue>", "")
            .replace("<dim>", "")
            .replace("</dim>", "");
        assert!(
            rows.iter().any(|row| row == plain.trim()),
            "authored row `{plain}` missing: {rows:#?}"
        );
    }
}
