#[cfg(test)]
mod tests {
    use darkmatter::markdown::{Markdown, output::{for_terminal, as_html, TerminalOptions}};
    use biscuit_terminal::terminal::Terminal;
    
    #[test]
    fn test_markdown_to_terminal_horizontal_rule() {
        let markdown = "--- { style: waves, placement: centered, weight: thick }";
        let md: Markdown = markdown.into();
        let result = for_terminal(&md, TerminalOptions::default()).unwrap();
        
        // Should contain the rendered horizontal rule
        assert!(!result.is_empty());
        // The result should contain wave characters (Unicode ≋ or ASCII ~)
        assert!(result.contains('≋') || result.contains('~'));
    }
    
    #[test]
    fn test_markdown_to_html_horizontal_rule() {
        let markdown = "--- { style: dots, width: \"50%\", color: \"red\" }";
        let md: Markdown = markdown.into();
        let result = as_html(&md, Default::default()).unwrap();
        
        // Should contain SVG with the specified attributes
        assert!(result.contains(r#"width="50%""#));
        assert!(result.contains(r#"stroke="red""#));
        assert!(result.contains(r#"stroke-width="4""#)); // dots default to medium weight in browser
    }
    
    #[test]
    fn test_markdown_with_multiple_horizontal_rules() {
        let markdown = "# Header\n\n--- { style: dashes }\n\nSome content\n\n*** { style: waves, placement: centered }\n\nMore content\n\n___ { style: dots, weight: thick, width: \"75%\" }\n";
        let md: Markdown = markdown.into();
        let terminal_result = for_terminal(&md, TerminalOptions::default()).unwrap();
        let html_result = as_html(&md, Default::default()).unwrap();
        
        // Should contain multiple horizontal rules
        assert!(terminal_result.contains('╌') || terminal_result.contains('≋') || terminal_result.contains('·') || terminal_result.contains('-'));
        assert!(html_result.contains("stroke-width=\"4\"") || html_result.contains("stroke-width=\"8\""));
    }
    
    #[test]
    fn test_horizontal_rule_in_complex_document() {
        let markdown = "# Complex Document\n\n## Section 1\n\nRegular paragraph with some text.\n\n--- { style: curtain-rod, placement: full }\n\n## Section 2\n\nAnother paragraph.\n\n*** { style: line-circle, placement: left, color: \"#00ff00\" }\n\n### Subsection\n\nFinal content.\n\n___ { style: inset-line, weight: medium, width: \"60%\" }\n";
        let md: Markdown = markdown.into();
        let terminal_result = for_terminal(&md, TerminalOptions::default()).unwrap();
        let html_result = as_html(&md, Default::default()).unwrap();
        
        // Should render without errors
        assert!(!terminal_result.is_empty());
        assert!(!html_result.is_empty());
        
        // Should contain expected elements
        assert!(terminal_result.contains('「') || terminal_result.contains('○') || terminal_result.contains('['));
        assert!(html_result.contains("currentColor") || html_result.contains("#00ff00"));
    }
    
    #[test]
    fn test_horizontal_rule_with_default_attributes() {
        let markdown = "--- { }";
        let md: Markdown = markdown.into();
        let terminal_result = for_terminal(&md, TerminalOptions::default()).unwrap();
        let html_result = as_html(&md, Default::default()).unwrap();
        
        // Should render with default attributes
        // Terminal uses Unicode dashes (╌) when color support is available
        assert!(terminal_result.contains('╌') || terminal_result.contains('-'));
        assert!(html_result.contains(r#"width="100%""#));
        assert!(html_result.contains(r#"stroke="currentColor""#));
        assert!(html_result.contains(r#"stroke-width="4""#)); // medium weight in browser
    }
    
    #[test]
    fn test_horizontal_rule_edge_cases() {
        // Test with various edge cases
        let test_cases = vec![
            "--- { style: dashes }",
            "*** { placement: centered }",
            "___ { weight: thin }",
            "--- { width: \"100%\" }",
            "*** { color: \"blue\" }",
        ];
        
        for markdown in test_cases {
            let md: Markdown = markdown.into();
            let terminal_result = for_terminal(&md, TerminalOptions::default()).unwrap();
        let html_result = as_html(&md, Default::default()).unwrap();
            
            assert!(!terminal_result.is_empty());
            assert!(!html_result.is_empty());
        }
    }
}