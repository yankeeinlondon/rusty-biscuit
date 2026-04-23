#[cfg(test)]
mod tests {
    use darkmatter::markdown::{
        Markdown,
        output::{TerminalOptions, as_html, for_terminal},
    };
    use insta::assert_snapshot;

    #[test]
    fn test_snapshot_markdown_to_terminal() {
        let test_cases = vec![
            ("simple_dashes", "--- { style: dashes }"),
            ("simple_dots", "--- { style: dots }"),
            ("simple_waves", "--- { style: waves }"),
            ("simple_line_star", "--- { style: line-star }"),
            ("simple_line_circle", "--- { style: line-circle }"),
            ("simple_inset_line", "--- { style: inset-line }"),
            ("simple_curtain_rod", "--- { style: curtain-rod }"),
            (
                "centered_waves",
                "--- { style: waves, placement: centered }",
            ),
            ("left_dots", "--- { style: dots, placement: left }"),
            ("right_dashes", "--- { style: dashes, placement: right }"),
            ("thick_waves", "--- { style: waves, weight: thick }"),
            ("thin_dots", "--- { style: dots, weight: thin }"),
            ("custom_width", "--- { style: waves, width: \"50%\" }"),
            ("custom_color", "--- { style: dots, color: \"red\" }"),
            (
                "all_attributes",
                "--- { style: waves, placement: centered, weight: thick, width: \"75%\", color: \"blue\" }",
            ),
        ];

        for (name, markdown) in test_cases {
            let md: Markdown = markdown.into();
            let result = for_terminal(&md, TerminalOptions::default()).unwrap();
            assert_snapshot!(format!("terminal_{}", name), result);
        }
    }

    #[test]
    fn test_snapshot_markdown_to_html() {
        let test_cases = vec![
            ("simple_dashes", "--- { style: dashes }"),
            ("simple_dots", "--- { style: dots }"),
            ("simple_waves", "--- { style: waves }"),
            (
                "centered_waves",
                "--- { style: waves, placement: centered }",
            ),
            ("thick_waves", "--- { style: waves, weight: thick }"),
            ("custom_width", "--- { style: waves, width: \"50%\" }"),
            ("custom_color", "--- { style: dots, color: \"red\" }"),
            (
                "all_attributes",
                "--- { style: waves, placement: centered, weight: thick, width: \"75%\", color: \"blue\" }",
            ),
        ];

        for (name, markdown) in test_cases {
            let md: Markdown = markdown.into();
            let result = as_html(&md, Default::default()).unwrap();
            assert_snapshot!(format!("html_{}", name), result);
        }
    }

    #[test]
    fn test_snapshot_complex_document() {
        let markdown = "# Horizontal Rule Examples\n\n## Basic Styles\n\n--- { style: dashes }\n\n*** { style: dots }\n\n___ { style: waves }\n\n## Placement Options\n\n--- { style: waves, placement: centered }\n\n*** { style: dots, placement: left }\n\n___ { style: dashes, placement: right }\n\n## Weight Variations\n\n--- { style: waves, weight: thin }\n\n*** { style: dots, weight: medium }\n\n___ { style: dashes, weight: thick }\n\n## Custom Attributes\n\n--- { style: waves, width: \"60%\", color: \"#ff0000\" }\n\n*** { style: dots, placement: centered, weight: thick, width: \"80%\", color: \"green\" }\n";

        let md: Markdown = markdown.into();
        let terminal_result = for_terminal(&md, TerminalOptions::default()).unwrap();
        let html_result = as_html(&md, Default::default()).unwrap();

        assert_snapshot!("terminal_complex_document", terminal_result);
        assert_snapshot!("html_complex_document", html_result);
    }
}
