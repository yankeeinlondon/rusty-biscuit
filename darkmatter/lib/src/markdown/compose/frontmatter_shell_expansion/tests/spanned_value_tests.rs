    use super::*;

    #[test]
    fn returns_none_for_non_shell_value() {
        assert!(parse_frontmatter_shell_value_spanned("plain text").is_none());
        assert!(parse_frontmatter_shell_value_spanned("").is_none());
    }

    #[test]
    fn parses_simple_pipeline_delimiters_and_tokens() {
        let value = "$(echo hi)";
        let parsed = parse_frontmatter_shell_value_spanned(value).unwrap();
        assert_eq!(&value[parsed.open_span.clone()], "$(");
        assert_eq!(&value[parsed.close_span.clone()], ")");
        assert_eq!(&value[parsed.inner_span.clone()], "echo hi");
        assert_eq!(parsed.span, 0..value.len());
        let FrontmatterShellBody::Pipeline(pipeline) = &parsed.body else {
            panic!("expected pipeline, got {:?}", parsed.body);
        };
        assert_eq!(pipeline.actions.len(), 1);
        let tokens = &pipeline.actions[0].tokens;
        assert_eq!(tokens.len(), 2);
        assert_eq!(&value[tokens[0].clone()], "echo");
        assert_eq!(&value[tokens[1].clone()], "hi");
    }

    #[test]
    fn leading_whitespace_is_reflected_in_spans() {
        let value = "  $(date)  ";
        let parsed = parse_frontmatter_shell_value_spanned(value).unwrap();
        assert_eq!(&value[parsed.open_span.clone()], "$(");
        assert_eq!(&value[parsed.inner_span.clone()], "date");
        assert_eq!(&value[parsed.close_span.clone()], ")");
    }

    #[test]
    fn parses_chain_operator_actions() {
        let value = "$(a && b || c)";
        let parsed = parse_frontmatter_shell_value_spanned(value).unwrap();
        let FrontmatterShellBody::Pipeline(pipeline) = &parsed.body else {
            panic!("expected pipeline");
        };
        assert_eq!(pipeline.actions.len(), 3);
        assert_eq!(&value[pipeline.actions[0].span.clone()], "a");
        assert_eq!(&value[pipeline.actions[1].span.clone()], "b");
        assert_eq!(&value[pipeline.actions[2].span.clone()], "c");
    }

    #[test]
    fn parses_ternary_branch_spans() {
        let value = "$( file_exists('x') ? cat x : echo none )";
        let parsed = parse_frontmatter_shell_value_spanned(value).unwrap();
        let FrontmatterShellBody::Ternary(t) = &parsed.body else {
            panic!("expected ternary, got {:?}", parsed.body);
        };
        assert_eq!(&value[t.condition_span.clone()], "file_exists('x')");
        assert_eq!(&value[t.then_span.clone()], "cat x");
        assert_eq!(&value[t.else_span.clone()], "echo none");
    }

    #[test]
    fn parses_suffix_spans() {
        let value = "$(slow)::timeout:30::no-cache";
        let parsed = parse_frontmatter_shell_value_spanned(value).unwrap();
        assert_eq!(parsed.suffixes.len(), 2);
        assert_eq!(parsed.suffixes[0].value, FrontmatterShellSuffix::Timeout(30));
        assert_eq!(&value[parsed.suffixes[0].span.clone()], "::timeout:30");
        assert_eq!(parsed.suffixes[1].value, FrontmatterShellSuffix::NoCache);
        assert_eq!(&value[parsed.suffixes[1].span.clone()], "::no-cache");
        // The whole-value span includes the suffixes.
        assert_eq!(parsed.span, 0..value.len());
    }

    #[test]
    fn unclosed_paren_yields_none() {
        assert!(parse_frontmatter_shell_value_spanned("$(echo hi").is_none());
    }

    #[test]
    fn nested_parens_do_not_close_early() {
        let value = "$(echo (a b))";
        let parsed = parse_frontmatter_shell_value_spanned(value).unwrap();
        assert_eq!(&value[parsed.inner_span.clone()], "echo (a b)");
    }
