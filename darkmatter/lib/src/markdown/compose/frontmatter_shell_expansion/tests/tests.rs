    use super::*;
    use serde_json::json;

    fn test_ctx() -> SourceContext {
        SourceContext::new(
            std::path::PathBuf::from("/test"),
            std::path::PathBuf::from("test"),
            String::new(),
        )
    }

    fn parse_shell_value(
        value: &str,
        key: &str,
        original_value: Option<&str>,
    ) -> Result<Option<FrontmatterShellDirective>, ShellExpansionError> {
        super::parse_shell_value(value, key, original_value, &test_ctx())
    }

    #[test]
    fn ternary_condition_uses_read_side_functions_with_context() {
        // A `$()` ternary condition evaluated at the real run carries the
        // resolution context, so `file_exists(...)` resolves against base_dir.
        use crate::markdown::compose::ComposeContext;
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();
        let rc = super::super::expression::ResolutionContext::new(dir.path().to_path_buf());
        let state = FrontmatterSeedState::new(
            std::collections::HashMap::new(),
            ComposeContext::fixed_for_testing(),
        )
        .with_resolution_context(Some(rc));

        assert!(
            evaluate_ternary_condition("file_exists('Cargo.toml')", &state, "k", &test_ctx())
                .unwrap()
        );
        assert!(
            !evaluate_ternary_condition("file_exists('nope.toml')", &state, "k", &test_ctx())
                .unwrap()
        );
    }

    #[test]
    fn ternary_condition_without_context_fails_loudly() {
        // The context-free seed state (preflight-style) cannot evaluate a
        // read-side function and surfaces a parse/eval error rather than
        // silently selecting a branch.
        use crate::markdown::compose::ComposeContext;
        let state = FrontmatterSeedState::new(
            std::collections::HashMap::new(),
            ComposeContext::fixed_for_testing(),
        );
        assert!(
            evaluate_ternary_condition("file_exists('Cargo.toml')", &state, "k", &test_ctx())
                .is_err()
        );
    }

    #[allow(dead_code)]
    fn scan_frontmatter(
        frontmatter: &Frontmatter,
        pre_interpolation_snapshot: Option<&std::collections::HashMap<String, String>>,
    ) -> Result<Vec<FrontmatterShellDirective>, ShellExpansionError> {
        super::scan_frontmatter(
            frontmatter,
            pre_interpolation_snapshot,
            &test_ctx(),
            &std::collections::HashSet::new(),
        )
    }

    #[allow(dead_code)]
    fn execute_frontmatter_shell_expansion(
        frontmatter: &mut Frontmatter,
        options: &ComposeOptions,
        runtime: &mut PipelineRuntime,
        pre_interpolation_snapshot: Option<&std::collections::HashMap<String, String>>,
    ) -> MarkdownResult<FrontmatterShellExpansionReport> {
        super::execute_frontmatter_shell_expansion(
            frontmatter,
            options,
            runtime,
            pre_interpolation_snapshot,
            &test_ctx(),
        )
    }

    #[test]
    fn detects_simple_shell_expression() {
        let result = parse_shell_value("$(echo hello)", "key", None);
        assert!(result.is_ok());
        let directive = result.unwrap().unwrap();
        assert_eq!(directive.executable, "echo");
        assert_eq!(directive.args, vec!["hello"]);
        assert_eq!(directive.raw_command, "echo hello");
        assert!(directive.timeout_override.is_none());
    }

    #[test]
    fn detects_expression_with_timeout() {
        let result = parse_shell_value("$(pwd)::timeout:3", "key", None);
        assert!(result.is_ok());
        let directive = result.unwrap().unwrap();
        assert_eq!(directive.executable, "pwd");
        assert_eq!(directive.args.len(), 0);
        assert_eq!(
            directive.timeout_override,
            Some(std::time::Duration::from_secs(3))
        );
    }

    #[test]
    fn ignores_non_shell_string() {
        let result = parse_shell_value("plain text", "key", None);
        assert!(matches!(result, Ok(None)));
    }

    #[test]
    fn ignores_partial_match_no_closing() {
        let result = parse_shell_value("$(echo hello", "key", None);
        assert!(result.is_err());
    }

    #[test]
    fn ignores_embedded_expression() {
        let result = parse_shell_value("prefix $(cmd) suffix", "key", None);
        assert!(matches!(result, Ok(None)));
    }

    #[test]
    fn rejects_zero_timeout() {
        let result = parse_shell_value("$(echo)::timeout:0", "key", None);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_non_integer_timeout() {
        let result = parse_shell_value("$(echo)::timeout:abc", "key", None);
        assert!(result.is_err());
    }

    #[test]
    fn detects_no_cache_suffix() {
        let directive = parse_shell_value("$(rustc)::no-cache", "key", None)
            .unwrap()
            .unwrap();
        assert_eq!(directive.executable, "rustc");
        assert!(directive.no_cache);
        assert!(directive.timeout_override.is_none());
    }

    #[test]
    fn no_cache_defaults_false_without_suffix() {
        let directive = parse_shell_value("$(rustc)", "key", None)
            .unwrap()
            .unwrap();
        assert!(!directive.no_cache);
    }

    #[test]
    fn no_cache_combines_with_timeout_either_order() {
        let a = parse_shell_value("$(rustc)::no-cache::timeout:5", "key", None)
            .unwrap()
            .unwrap();
        assert!(a.no_cache);
        assert_eq!(a.timeout_override, Some(std::time::Duration::from_secs(5)));

        let b = parse_shell_value("$(rustc)::timeout:5::no-cache", "key", None)
            .unwrap()
            .unwrap();
        assert!(b.no_cache);
        assert_eq!(b.timeout_override, Some(std::time::Duration::from_secs(5)));
    }

    #[test]
    fn rejects_invalid_suffix_after_expression() {
        let result = parse_shell_value("$(uuidgen)::bogus", "key", None);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unexpected trailing content")
        );
    }

    #[test]
    fn rejects_duplicate_no_cache_suffix() {
        let result = parse_shell_value("$(uuidgen)::no-cache::no-cache", "key", None);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_interpolated_executable() {
        let original = "$({{cmd}} arg)";
        let resolved = "$(ls arg)";
        let result = parse_shell_value(resolved, "key", Some(original));
        assert!(result.is_err());
    }

    #[test]
    fn rejects_interpolated_executable_after_or_operator() {
        let original = "$(false || {{cmd}} arg)";
        let resolved = "$(false || echo arg)";
        let err = parse_shell_value(resolved, "key", Some(original)).unwrap_err();
        match err {
            ShellExpansionError::ParseDirective { message, .. } => {
                assert!(
                    message.contains("may not come from interpolation"),
                    "unexpected message: {message}"
                );
            }
            other => panic!("Expected ParseDirective, got {other:?}"),
        }
    }

    #[test]
    fn rejects_interpolated_executable_after_and_operator() {
        let original = "$(true && {{cmd}} arg)";
        let resolved = "$(true && echo arg)";
        let err = parse_shell_value(resolved, "key", Some(original)).unwrap_err();
        match err {
            ShellExpansionError::ParseDirective { message, .. } => {
                assert!(
                    message.contains("may not come from interpolation"),
                    "unexpected message: {message}"
                );
            }
            other => panic!("Expected ParseDirective, got {other:?}"),
        }
    }

    #[test]
    fn rejects_interpolated_executable_in_third_chain_segment() {
        let original = "$(true && false || {{cmd}} arg)";
        let resolved = "$(true && false || echo arg)";
        let result = parse_shell_value(resolved, "key", Some(original));
        assert!(result.is_err());
    }

    #[test]
    fn accepts_interpolated_argument_after_chain_operator() {
        let original = "$(false || echo {{file}})";
        let resolved = "$(false || echo README.md)";
        let result = parse_shell_value(resolved, "key", Some(original));
        assert!(result.is_ok());
    }

    #[test]
    fn ignores_chain_operators_inside_quotes() {
        // `||` inside single quotes is literal, not a chain operator,
        // so the interpolation here is in argument position, not executable.
        let original = "$(echo 'a || {{cmd}}')";
        let resolved = "$(echo 'a || hello')";
        let result = parse_shell_value(resolved, "key", Some(original));
        assert!(result.is_ok());
    }

    #[test]
    fn accepts_interpolated_argument() {
        let original = "$(dirname {{file}})";
        let resolved = "$(dirname README.md)";
        let result = parse_shell_value(resolved, "key", Some(original));
        assert!(result.is_ok());
        let directive = result.unwrap().unwrap();
        assert_eq!(directive.executable, "dirname");
        assert_eq!(directive.args, vec!["README.md"]);
    }

    #[test]
    fn accepts_no_interpolation_at_all() {
        let original = "$(echo hello)";
        let result = parse_shell_value(original, "key", Some(original));
        assert!(result.is_ok());
        let directive = result.unwrap().unwrap();
        assert_eq!(directive.executable, "echo");
        assert_eq!(directive.args, vec!["hello"]);
    }

    #[test]
    fn accepts_closing_paren_inside_quoted_argument() {
        let result = parse_shell_value("$(printf ')')", "key", None).unwrap();
        let directive = result.unwrap();
        assert_eq!(directive.executable, "printf");
        assert_eq!(directive.args, vec![")"]);
    }

    #[test]
    fn scan_finds_shell_in_top_level_strings() {
        let mut fm = Frontmatter::new();
        fm.insert("cmd1", json!("$(echo hello)")).unwrap();
        fm.insert("plain", json!("not a shell command")).unwrap();
        fm.insert("cmd2", json!("$(pwd)")).unwrap();
        fm.insert("number", json!(42)).unwrap();

        let directives = scan_frontmatter(&fm, None).unwrap();
        assert_eq!(directives.len(), 2);
        assert!(directives.iter().any(|d| d.key == "cmd1"));
        assert!(directives.iter().any(|d| d.key == "cmd2"));
    }

    #[test]
    fn scan_skips_nested_objects() {
        let mut fm = Frontmatter::new();
        fm.insert("outer", json!({"inner": "$(echo nested)"}))
            .unwrap();
        fm.insert("top", json!("$(echo top)")).unwrap();

        let directives = scan_frontmatter(&fm, None).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].key, "top");
    }

    #[test]
    fn scan_skips_arrays() {
        let mut fm = Frontmatter::new();
        fm.insert("arr", json!(["$(echo one)", "$(echo two)"]))
            .unwrap();
        fm.insert("top", json!("$(echo top)")).unwrap();

        let directives = scan_frontmatter(&fm, None).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].key, "top");
    }

    #[test]
    fn scan_errors_on_malformed_shell_expression() {
        let mut fm = Frontmatter::new();
        fm.insert("bad", json!("$(echo hi)::timeout:0")).unwrap();

        let err = scan_frontmatter(&fm, None).unwrap_err();
        match err {
            ShellExpansionError::ParseDirective { origin, .. } => {
                assert_eq!(
                    origin,
                    ShellCommandOrigin::Frontmatter {
                        key: "bad".to_string(),
                        line: None,
                    }
                );
            }
            other => panic!("Expected ParseDirective, got {other:?}"),
        }
    }

    #[test]
    fn leak_guard_rejects_surviving_whole_value_candidate() {
        // A value that is still a clean whole-value `$(...)` after expansion —
        // e.g. command output that reproduced `$( … )` — is rejected.
        let mut fm = Frontmatter::new();
        fm.insert("leaked", json!("$(echo hi)")).unwrap();

        let err = super::validate_no_whole_value_shell_leak(&fm, &test_ctx(), &std::collections::HashSet::new()).unwrap_err();
        match err {
            ShellExpansionError::ParseDirective { origin, message, .. } => {
                assert_eq!(
                    origin,
                    ShellCommandOrigin::Frontmatter {
                        key: "leaked".to_string(),
                        line: None,
                    }
                );
                assert!(
                    message.contains("survived shell expansion"),
                    "unexpected message: {message}"
                );
            }
            other => panic!("Expected ParseDirective, got {other:?}"),
        }
    }

    #[test]
    fn leak_guard_trims_before_classifying() {
        // A whole-value `$(...)` behind leading whitespace is skipped by the
        // strict-start scan but is still a leak after trimming.
        let mut fm = Frontmatter::new();
        fm.insert("padded", json!("   $(echo hi)  ")).unwrap();
        assert!(super::validate_no_whole_value_shell_leak(&fm, &test_ctx(), &std::collections::HashSet::new()).is_err());
    }

    #[test]
    fn leak_guard_ignores_plain_and_mixed_values() {
        // Plain text, mixed literals, and `$(...)` with trailing content are not
        // whole-value candidates and pass the guard untouched.
        let mut fm = Frontmatter::new();
        fm.insert("plain", json!("just text")).unwrap();
        fm.insert("mixed_prefix", json!("literal $(echo ok)")).unwrap();
        fm.insert("mixed_suffix", json!("$(echo ok) trailing")).unwrap();
        fm.insert("expanded", json!("hello")).unwrap();
        fm.insert("number", json!(42)).unwrap();

        assert!(super::validate_no_whole_value_shell_leak(&fm, &test_ctx(), &std::collections::HashSet::new()).is_ok());
    }

    #[test]
    fn leak_guard_rejects_padded_malformed_whole_value() {
        // A padded whole-value `$(...)` shape that fails to close is a fatal
        // error, not a silent leak. The strict-start scan skips it (leading
        // whitespace), so the guard is the only line of defense.
        let mut fm = Frontmatter::new();
        fm.insert("padded", json!("  $(echo ok")).unwrap();

        let err = super::validate_no_whole_value_shell_leak(&fm, &test_ctx(), &std::collections::HashSet::new()).unwrap_err();
        match err {
            ShellExpansionError::ParseDirective { origin, message, .. } => {
                assert_eq!(
                    origin,
                    ShellCommandOrigin::Frontmatter {
                        key: "padded".to_string(),
                        line: None,
                    }
                );
                // The propagated diagnostic is the missing-paren parse error,
                // NOT the generic "survived shell expansion" leak message.
                assert!(
                    message.contains("Missing closing ')'"),
                    "unexpected message: {message}"
                );
            }
            other => panic!("Expected ParseDirective, got {other:?}"),
        }
    }

    #[test]
    fn leak_guard_rejects_padded_no_command_whole_value() {
        // A padded whole-value `$(...)` whose body is a no-command expression
        // (`file_exists('x')`) is a fatal error, propagating the no-command
        // diagnostic rather than leaking.
        let mut fm = Frontmatter::new();
        fm.insert("padded", json!("  $(file_exists('x'))")).unwrap();

        let err = super::validate_no_whole_value_shell_leak(&fm, &test_ctx(), &std::collections::HashSet::new()).unwrap_err();
        match err {
            ShellExpansionError::ParseDirective { origin, .. } => {
                assert_eq!(
                    origin,
                    ShellCommandOrigin::Frontmatter {
                        key: "padded".to_string(),
                        line: None,
                    }
                );
            }
            other => panic!("Expected ParseDirective, got {other:?}"),
        }
    }

    #[test]
    fn leak_guard_padded_trailing_literal_stays_lenient() {
        // A padded `$(...)` with non-suffix trailing content is a trailing-literal
        // mixed value, not a whole-value shape — it stays lenient.
        let mut fm = Frontmatter::new();
        fm.insert("padded", json!("  $(echo ok) trailing")).unwrap();
        assert!(super::validate_no_whole_value_shell_leak(&fm, &test_ctx(), &std::collections::HashSet::new()).is_ok());
    }

    #[test]
    fn is_whole_value_shell_candidate_recognizes_forms() {
        use super::WholeValueShellShape;
        let ctx = test_ctx();
        // Clean whole-value forms (incl. padded-valid) are fatal-leak candidates.
        assert!(matches!(
            super::is_whole_value_shell_candidate("$(echo hi)", "k", &ctx),
            WholeValueShellShape::CleanDirective
        ));
        assert!(matches!(
            super::is_whole_value_shell_candidate("  $(pwd)  ", "k", &ctx),
            WholeValueShellShape::CleanDirective
        ));
        // Malformed and no-command whole-value shapes are now recognized as
        // fatal shape-but-error candidates (the bug fix).
        assert!(matches!(
            super::is_whole_value_shell_candidate("  $(echo ok", "k", &ctx),
            WholeValueShellShape::ShapeButError(_)
        ));
        assert!(matches!(
            super::is_whole_value_shell_candidate("  $(file_exists('x'))", "k", &ctx),
            WholeValueShellShape::ShapeButError(_)
        ));
        // Not a candidate: plain text, mixed literal, trailing content.
        assert!(matches!(
            super::is_whole_value_shell_candidate("plain", "k", &ctx),
            WholeValueShellShape::NotCandidate
        ));
        assert!(matches!(
            super::is_whole_value_shell_candidate("x $(echo ok)", "k", &ctx),
            WholeValueShellShape::NotCandidate
        ));
        assert!(matches!(
            super::is_whole_value_shell_candidate("$(echo ok) x", "k", &ctx),
            WholeValueShellShape::NotCandidate
        ));
    }

    fn unwrap_ternary(
        directive: &FrontmatterShellDirective,
    ) -> (&str, &Branch, &Branch) {
        match &directive.ast {
            FrontmatterShellAst::Ternary {
                condition_source,
                then_branch,
                else_branch,
            } => (condition_source.as_str(), then_branch, else_branch),
            FrontmatterShellAst::Pipeline(_) => {
                panic!("expected Ternary AST, got Pipeline")
            }
        }
    }

    #[test]
    fn split_top_level_ternary_basic() {
        let (cond, then_s, else_s) =
            super::split_top_level_ternary("{{has_spec}} ? basename '{{spec}}' : ''")
                .expect("ternary should split");
        assert_eq!(cond.trim(), "{{has_spec}}");
        assert_eq!(then_s.trim(), "basename '{{spec}}'");
        assert_eq!(else_s.trim(), "''");
    }

    #[test]
    fn split_top_level_ternary_quotes_protect_punctuation() {
        // `?` and `:` inside single/double quotes are not top-level.
        assert!(super::split_top_level_ternary("echo 'is it?'").is_none());
        assert!(super::split_top_level_ternary("echo \"a : b\"").is_none());
    }

    #[test]
    fn split_top_level_ternary_parens_protect_punctuation() {
        // Parenthesized sub-expression in the condition is masked.
        let (cond, then_s, else_s) =
            super::split_top_level_ternary("(a ? b) ? then_cmd : else_cmd")
                .expect("outer ternary should split");
        assert_eq!(cond.trim(), "(a ? b)");
        assert_eq!(then_s.trim(), "then_cmd");
        assert_eq!(else_s.trim(), "else_cmd");
    }

    #[test]
    fn split_top_level_ternary_question_without_colon_returns_none() {
        assert!(super::split_top_level_ternary("a ? b").is_none());
    }

    #[test]
    fn parses_basic_ternary() {
        let original = "$({{has_spec}} ? basename '{{spec}}' : '')";
        let resolved = "$(true ? basename '/tmp/spec.md' : '')";
        let directive = parse_shell_value(resolved, "spec_file", Some(original))
            .expect("parse should succeed")
            .expect("directive should be returned");
        let (cond, then_b, else_b) = unwrap_ternary(&directive);
        assert_eq!(cond, "{{has_spec}}");
        // Branches now carry the ORIGINAL slice; resolved text is produced
        // by per-branch interpolation at execute time.
        match then_b {
            Branch::Pipeline { original_text } => {
                assert_eq!(original_text.trim(), "basename '{{spec}}'");
            }
            _ => panic!("expected then-branch pipeline"),
        }
        assert!(matches!(else_b, Branch::Empty));
        assert!(directive.pipeline.is_none());
    }

    #[test]
    fn parses_ternary_with_both_pipeline_branches() {
        let inner = "$(cond ? echo yes : echo no)";
        let directive = parse_shell_value(inner, "key", None)
            .unwrap()
            .expect("directive should parse");
        let (cond, then_b, else_b) = unwrap_ternary(&directive);
        assert_eq!(cond, "cond");
        match (then_b, else_b) {
            (
                Branch::Pipeline { original_text: then_text },
                Branch::Pipeline { original_text: else_text },
            ) => {
                assert_eq!(then_text.trim(), "echo yes");
                assert_eq!(else_text.trim(), "echo no");
            }
            _ => panic!("expected both branches to be pipelines"),
        }
    }

    #[test]
    fn parses_ternary_with_double_quoted_empty_branch() {
        let inner = r#"$(cond ? echo yes : "")"#;
        let directive = parse_shell_value(inner, "key", None)
            .unwrap()
            .expect("directive should parse");
        let (_cond, _then_b, else_b) = unwrap_ternary(&directive);
        assert!(matches!(else_b, Branch::Empty));
    }

    #[test]
    fn ternary_question_without_colon_errors() {
        let inner = "$({{cond}} ? echo yes)";
        let err = parse_shell_value(inner, "key", None).unwrap_err();
        match err {
            ShellExpansionError::ParseDirective { message, .. } => {
                assert!(
                    message.contains("missing ':'"),
                    "unexpected message: {message}"
                );
            }
            other => panic!("Expected ParseDirective, got {other:?}"),
        }
    }

    #[test]
    fn ternary_rejects_interpolated_executable_in_then_branch() {
        let original = "$(cond ? {{cmd}} arg : '')";
        let resolved = "$(cond ? echo arg : '')";
        let err = parse_shell_value(resolved, "key", Some(original)).unwrap_err();
        match err {
            ShellExpansionError::ParseDirective { message, .. } => {
                assert!(
                    message.contains("may not come from interpolation"),
                    "unexpected message: {message}"
                );
                assert!(
                    message.contains("then-branch"),
                    "expected branch label in message: {message}"
                );
            }
            other => panic!("Expected ParseDirective, got {other:?}"),
        }
    }

    #[test]
    fn ternary_rejects_interpolated_executable_in_else_branch() {
        let original = "$(cond ? echo yes : {{cmd}} arg)";
        let resolved = "$(cond ? echo yes : echo arg)";
        let err = parse_shell_value(resolved, "key", Some(original)).unwrap_err();
        match err {
            ShellExpansionError::ParseDirective { message, .. } => {
                assert!(
                    message.contains("may not come from interpolation"),
                    "unexpected message: {message}"
                );
                assert!(
                    message.contains("else-branch"),
                    "expected branch label in message: {message}"
                );
            }
            other => panic!("Expected ParseDirective, got {other:?}"),
        }
    }

    #[test]
    fn ternary_allows_interpolated_argument_in_branch() {
        let original = "$(cond ? basename {{file}} : '')";
        let resolved = "$(cond ? basename README.md : '')";
        let directive = parse_shell_value(resolved, "key", Some(original))
            .unwrap()
            .expect("directive should parse");
        let (_, then_b, _) = unwrap_ternary(&directive);
        match then_b {
            Branch::Pipeline { original_text } => {
                // Branch retains the original {{file}} placeholder; expansion
                // happens at execute time via per-branch interpolation.
                assert_eq!(original_text.trim(), "basename {{file}}");
            }
            _ => panic!("expected pipeline branch"),
        }
    }

    #[test]
    fn ternary_empty_condition_errors() {
        let inner = "$( ? echo yes : '')";
        let err = parse_shell_value(inner, "key", None).unwrap_err();
        match err {
            ShellExpansionError::ParseDirective { message, .. } => {
                assert!(
                    message.contains("condition"),
                    "unexpected message: {message}"
                );
            }
            other => panic!("Expected ParseDirective, got {other:?}"),
        }
    }

    #[test]
    fn ternary_branch_command_pipeline_with_chain_operators() {
        let inner = "$(cond ? echo a && echo b : echo c)";
        let directive = parse_shell_value(inner, "key", None)
            .unwrap()
            .expect("directive should parse");
        let (_, then_b, _) = unwrap_ternary(&directive);
        match then_b {
            Branch::Pipeline { original_text } => {
                assert_eq!(original_text.trim(), "echo a && echo b");
            }
            _ => panic!("expected pipeline branch"),
        }
    }

    #[test]
    fn ternary_quote_protected_punctuation_is_not_split() {
        // The `?` is inside single quotes — the directive is a plain pipeline.
        let inner = "$(echo 'is it?')";
        let directive = parse_shell_value(inner, "key", None)
            .unwrap()
            .expect("directive should parse");
        match directive.ast {
            FrontmatterShellAst::Pipeline(_) => {}
            FrontmatterShellAst::Ternary { .. } => {
                panic!("expected Pipeline AST, got Ternary")
            }
        }
    }

    #[test]
    fn ternary_rejects_nested_in_then_branch() {
        // Review finding 3: a second top-level `?` (here turning the then-
        // branch into another ternary `b ? echo two : echo three`) must be
        // refused at parse time rather than silently tokenized.
        let inner = "$(a ? b ? echo two : echo three : echo c)";
        let err = parse_shell_value(inner, "key", None).unwrap_err();
        match err {
            ShellExpansionError::ParseDirective { message, .. } => {
                assert!(
                    message.contains("nested ternaries are not supported")
                        && message.contains("additional separator-style '?'")
                        && message.contains("then-branch"),
                    "unexpected message: {message}"
                );
            }
            other => panic!("Expected ParseDirective, got {other:?}"),
        }
    }

    #[test]
    fn ternary_rejects_nested_in_else_branch() {
        // Review finding 3: a second top-level `?` in the else-branch is
        // also rejected.
        let inner = "$(a ? echo one : b ? echo two : echo three)";
        let err = parse_shell_value(inner, "key", None).unwrap_err();
        match err {
            ShellExpansionError::ParseDirective { message, .. } => {
                assert!(
                    message.contains("nested ternaries are not supported")
                        && message.contains("additional separator-style '?'")
                        && message.contains("else-branch"),
                    "unexpected message: {message}"
                );
            }
            other => panic!("Expected ParseDirective, got {other:?}"),
        }
    }

    #[test]
    fn ternary_accepts_bare_colon_in_else_branch_argument() {
        // A bare top-level `:` in a branch is valid pipeline content —
        // URLs and key:value arguments commonly contain it — so it must
        // not be rejected as a nested ternary. After the outer split, the
        // else-branch text `echo two : echo three` parses as a single
        // command with three arguments.
        let inner = "$(a ? echo one : echo two : echo three)";
        let directive = parse_shell_value(inner, "key", None)
            .unwrap()
            .expect("directive should parse");
        let (_, _, else_b) = unwrap_ternary(&directive);
        match else_b {
            Branch::Pipeline { original_text } => {
                assert_eq!(original_text.trim(), "echo two : echo three");
            }
            _ => panic!("expected pipeline else-branch"),
        }
    }

    #[test]
    fn ternary_accepts_url_with_colon_in_then_branch() {
        // Review finding 2: `:` inside a URL argument is normal pipeline
        // content and must round-trip through both branches.
        let inner = "$(flag ? echo http://example.com : echo none)";
        let directive = parse_shell_value(inner, "key", None)
            .unwrap()
            .expect("directive should parse");
        let (_, then_b, _) = unwrap_ternary(&directive);
        match then_b {
            Branch::Pipeline { original_text } => {
                assert_eq!(original_text.trim(), "echo http://example.com");
            }
            _ => panic!("expected pipeline then-branch"),
        }
    }

    #[test]
    fn ternary_accepts_url_with_colon_in_else_branch() {
        let inner = "$(flag ? echo none : echo http://example.com)";
        let directive = parse_shell_value(inner, "key", None)
            .unwrap()
            .expect("directive should parse");
        let (_, _, else_b) = unwrap_ternary(&directive);
        match else_b {
            Branch::Pipeline { original_text } => {
                assert_eq!(original_text.trim(), "echo http://example.com");
            }
            _ => panic!("expected pipeline else-branch"),
        }
    }

    #[test]
    fn ternary_branch_boundaries_pin_to_original_snapshot() {
        // Review finding 1: when an interpolated condition introduces
        // top-level `?` / `:` punctuation, the resolved inner naively
        // re-split would shift branch boundaries and let condition text
        // bleed into the then-branch executable. Anchoring the AST to the
        // original snapshot keeps the then-branch text statically equal to
        // what the author wrote.
        let original = "$({{cond}} ? basename README.md : '')";
        // Suppose `cond` was rendered to the literal text `true ? date : false`
        // by an earlier interpolation pass.
        let resolved = "$(true ? date : false ? basename README.md : '')";
        let directive = parse_shell_value(resolved, "key", Some(original))
            .expect("parse should succeed")
            .expect("directive should be returned");
        let (cond, then_b, else_b) = unwrap_ternary(&directive);
        assert_eq!(cond, "{{cond}}");
        match then_b {
            Branch::Pipeline { original_text } => {
                // Critically, the then-branch text is the ORIGINAL slice
                // (`basename README.md`), NOT the resolved slice (`date`)
                // that a naive resplit would have produced.
                assert_eq!(original_text.trim(), "basename README.md");
            }
            _ => panic!("expected pipeline then-branch"),
        }
        assert!(matches!(else_b, Branch::Empty));
    }

    #[test]
    fn ternary_separator_requires_whitespace_padding() {
        // Review-3 medium finding: the separator detection contract requires
        // whitespace padding on both sides of `?` and `:`. An unpadded `?`
        // like `flag? echo yes : ''` is parsed as part of the executable
        // token (`flag?`) — not as a ternary separator. This lock-in test
        // documents that the implementation matches the spec's
        // whitespace-padded rule rather than a raw "any top-level `?`" rule.
        let inner = "$(flag? echo yes : '')";
        let directive = parse_shell_value(inner, "key", None)
            .unwrap()
            .expect("directive should parse");
        match directive.ast {
            FrontmatterShellAst::Pipeline(ref pipeline) => {
                assert_eq!(pipeline.actions[0].command.executable, "flag?");
            }
            FrontmatterShellAst::Ternary { .. } => {
                panic!("expected Pipeline AST, got Ternary — separator must require whitespace padding")
            }
        }
    }

    #[test]
    fn ternary_with_timeout_preserves_timeout() {
        let inner = "$(cond ? echo yes : '')::timeout:5";
        let directive = parse_shell_value(inner, "key", None)
            .unwrap()
            .expect("directive should parse");
        assert!(matches!(directive.ast, FrontmatterShellAst::Ternary { .. }));
        assert_eq!(
            directive.timeout_override,
            Some(std::time::Duration::from_secs(5))
        );
    }

    // ── §2 token-resolution ladder ────────────────────────────────────────

    /// A name guaranteed not to resolve on `PATH`, used to exercise the
    /// bare-name → frontmatter-property rung of the ladder.
    const ABSENT_ON_PATH: &str = "dm_definitely_not_a_real_binary_xyz";

    #[test]
    fn classify_quoted_numeric_and_boolean_literals_are_values() {
        assert_eq!(super::classify_executed_body("'hello'"), BodyClass::Value);
        assert_eq!(super::classify_executed_body("\"hello\""), BodyClass::Value);
        assert_eq!(super::classify_executed_body("42"), BodyClass::Value);
        assert_eq!(super::classify_executed_body("3.14"), BodyClass::Value);
        assert_eq!(super::classify_executed_body("true"), BodyClass::Value);
        assert_eq!(super::classify_executed_body("false"), BodyClass::Value);
    }

    #[test]
    fn classify_true_false_are_never_commands_even_when_on_path() {
        // `true` and `false` are real executables on most systems, but the
        // ladder pins them to the boolean literal.
        assert_eq!(super::classify_executed_body("true"), BodyClass::Value);
        assert_eq!(super::classify_executed_body("false"), BodyClass::Value);
    }

    #[test]
    fn classify_expression_function_is_a_safe_value() {
        // Trailing parentheses mark a safe expression function — no shell
        // executable can contain `(`/`)`.
        assert_eq!(
            super::classify_executed_body("file_exists('Cargo.toml')"),
            BodyClass::Value
        );
        assert_eq!(
            super::classify_executed_body("markdown_title('a', 'b')"),
            BodyClass::Value
        );
    }

    #[test]
    fn classify_path_bearing_token_is_always_a_command() {
        // Path-bearing tokens are executables, never properties — even when no
        // such file exists.
        assert_eq!(
            super::classify_executed_body("/usr/bin/doit"),
            BodyClass::Command
        );
        assert_eq!(super::classify_executed_body("./doit"), BodyClass::Command);
    }

    #[test]
    fn classify_bare_name_on_path_is_a_command() {
        // `echo` is universally present.
        assert_eq!(super::classify_executed_body("echo"), BodyClass::Command);
    }

    #[test]
    fn classify_bare_name_not_on_path_is_a_property_value() {
        assert_eq!(
            super::classify_executed_body(ABSENT_ON_PATH),
            BodyClass::Value
        );
    }

    #[test]
    fn classify_doc_namespace_is_always_a_property_value() {
        // `doc.*` resolves the frontmatter property even when a same-named
        // executable exists on `PATH`.
        assert_eq!(super::classify_executed_body("doc.echo"), BodyClass::Value);
        assert_eq!(super::classify_executed_body("doc.build"), BodyClass::Value);
        assert_eq!(super::classify_executed_body("doc"), BodyClass::Value);
    }

    #[test]
    fn classify_multi_token_body_is_a_command() {
        assert_eq!(
            super::classify_executed_body("echo hello"),
            BodyClass::Command
        );
        assert_eq!(
            super::classify_executed_body("sniff repo dirty-files"),
            BodyClass::Command
        );
    }

    #[test]
    fn classify_empty_string_literal_is_empty() {
        assert_eq!(super::classify_executed_body("''"), BodyClass::Empty);
        assert_eq!(super::classify_executed_body("\"\""), BodyClass::Empty);
    }

    #[test]
    fn non_ternary_all_expression_value_errors_with_brace_suggestion() {
        // A bare `$()` that resolves to a value (here an expression function)
        // is a user error — steer them toward `{{ … }}`.
        let err = parse_shell_value("$(file_exists('x'))", "spec", None).unwrap_err();
        match err {
            ShellExpansionError::ParseDirective { message, .. } => {
                assert!(
                    message.contains("no shell command") && message.contains("{{"),
                    "expected a {{{{ }}}} suggestion, got: {message}"
                );
            }
            other => panic!("Expected ParseDirective, got {other:?}"),
        }
    }

    #[test]
    fn ternary_with_no_command_branch_errors_with_brace_suggestion() {
        // Both branches are string literals; the condition is an expression
        // function — the whole `$()` is expression content with no command.
        let err = parse_shell_value("$( file_exists('x') ? 'a' : 'b' )", "spec", None)
            .unwrap_err();
        match err {
            ShellExpansionError::ParseDirective { message, .. } => {
                assert!(
                    message.contains("no shell command") && message.contains("{{"),
                    "expected a {{{{ }}}} suggestion, got: {message}"
                );
            }
            other => panic!("Expected ParseDirective, got {other:?}"),
        }
    }

    #[test]
    fn ternary_value_branch_is_classified_as_value_not_pipeline() {
        // A literal fallback branch is a value, not a command pipeline; the
        // command branch keeps the directive valid.
        let directive = parse_shell_value("$(flag ? echo run : 'fallback')", "k", None)
            .unwrap()
            .expect("directive should parse");
        let (_, then_b, else_b) = unwrap_ternary(&directive);
        assert!(matches!(then_b, Branch::Pipeline { .. }));
        match else_b {
            Branch::Value { source } => assert_eq!(source.trim(), "'fallback'"),
            other => panic!("expected a value else-branch, got {other:?}"),
        }
    }

    #[test]
    fn mixed_expression_condition_with_command_branches_parses() {
        // The spec's intermixing example: the condition is expression content
        // (`file_exists(...)`) and both branches are real shell pipelines.
        let directive =
            parse_shell_value(
                "$( file_exists('Cargo.toml') ? cargo build : cargo test )",
                "k",
                None,
            )
                .unwrap()
                .expect("directive should parse");
        let (cond, then_b, else_b) = unwrap_ternary(&directive);
        assert_eq!(cond, "file_exists('Cargo.toml')");
        assert!(matches!(then_b, Branch::Pipeline { .. }));
        assert!(matches!(else_b, Branch::Pipeline { .. }));
    }
