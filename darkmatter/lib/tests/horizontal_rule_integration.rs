#[cfg(test)]
mod tests {
    use biscuit_terminal::terminal::Terminal;
    use darkmatter::layout::DarkmatterPage;
    use darkmatter::markdown::{
        Markdown,
        output::{ColorDepth, TerminalImageMode, TerminalOptions},
    };
    use darkmatter::style::{HrStyleOverrides, apply_hr_style, from_frontmatter};
    use test_toolkit::EnvGuard;

    fn terminal_text_options() -> TerminalOptions {
        let mut options = TerminalOptions::default();
        options.image_mode = TerminalImageMode::Never;
        options
    }

    #[test]
    fn test_markdown_to_terminal_horizontal_rule() {
        let markdown = "--- { style: waves, alignment: centered, weight: thick }";
        let md: Markdown = markdown.into();
        let result = md.as_terminal(terminal_text_options()).unwrap();

        // Should contain the rendered horizontal rule
        assert!(!result.is_empty());
        // The result should contain wave characters (Unicode ≋ or ASCII ~)
        assert!(result.contains('≋') || result.contains('~'));
    }

    #[test]
    fn test_markdown_to_html_horizontal_rule() {
        let markdown = "--- { style: dots, width: \"50%\", color: \"red\" }";
        let md: Markdown = markdown.into();
        let result = md.as_html(Default::default()).unwrap();

        // Outer <svg> keeps concrete `width="…"` for renderer compatibility.
        assert!(result.contains(r#"width="50%""#));
        // Phase 3 (A4): stroke and stroke-width flow through CSS variables
        // so callers can override them; the embedded fallback preserves
        // visual fidelity when inline styles are stripped.
        assert!(
            result.contains(r#"stroke="var(--hr-color, red)""#),
            "expected var(--hr-color, red): {result}"
        );
        assert!(
            result.contains(r#"stroke-width="var(--hr-weight, 4)""#),
            "expected var(--hr-weight, 4) (dots default to medium weight): {result}"
        );
        // The `--hr-*` custom properties are declared on the root <svg>.
        assert!(result.contains("--hr-color: red"));
        assert!(result.contains("--hr-weight: 4"));
        assert!(result.contains("--hr-width: 50%"));
    }

    #[test]
    fn test_markdown_with_multiple_horizontal_rules() {
        let markdown = "# Header\n\n--- { style: dashes }\n\nSome content\n\n*** { style: waves, alignment: centered }\n\nMore content\n\n___ { style: dots, weight: thick, width: \"75%\" }\n";
        let md: Markdown = markdown.into();
        let terminal_result = md.as_terminal(terminal_text_options()).unwrap();
        let html_result = md.as_html(Default::default()).unwrap();

        // Should contain multiple horizontal rules
        assert!(
            terminal_result.contains('╌')
                || terminal_result.contains('≋')
                || terminal_result.contains('·')
                || terminal_result.contains('-')
        );
        // Phase 3 (A4): stroke-width now goes through `var(--hr-weight, N)`.
        assert!(
            html_result.contains("stroke-width=\"var(--hr-weight, 4)\"")
                || html_result.contains("stroke-width=\"var(--hr-weight, 8)\"")
        );
    }

    #[test]
    fn test_horizontal_rule_in_complex_document() {
        let markdown = "# Complex Document\n\n## Section 1\n\nRegular paragraph with some text.\n\n--- { style: curtain-rod, alignment: full }\n\n## Section 2\n\nAnother paragraph.\n\n*** { style: line-circle, alignment: left, color: \"#00ff00\" }\n\n### Subsection\n\nFinal content.\n\n___ { style: inset-line, weight: medium, width: \"60%\" }\n";
        let md: Markdown = markdown.into();
        let terminal_result = md.as_terminal(terminal_text_options()).unwrap();
        let html_result = md.as_html(Default::default()).unwrap();

        // Should render without errors
        assert!(!terminal_result.is_empty());
        assert!(!html_result.is_empty());

        // Should contain expected elements. Curtain-rod now uses the
        // single-width box-drawing tees `┤` / `├` (see B8 fix). ASCII
        // fallback still uses `[` / `]`.
        assert!(
            terminal_result.contains('┤')
                || terminal_result.contains('├')
                || terminal_result.contains('●')
                || terminal_result.contains('[')
        );
        assert!(html_result.contains("currentColor") || html_result.contains("#00ff00"));
    }

    #[test]
    fn test_horizontal_rule_with_default_attributes() {
        let markdown = "--- { }";
        let md: Markdown = markdown.into();
        let terminal_result = md.as_terminal(terminal_text_options()).unwrap();
        let html_result = md.as_html(Default::default()).unwrap();

        // Should render with default attributes
        // Terminal uses Unicode dashes (╌) when color support is available
        assert!(terminal_result.contains('╌') || terminal_result.contains('-'));
        assert!(html_result.contains(r#"width="100%""#));
        // Phase 3 (A4): default color flows through var(--hr-color, currentColor)
        // and stroke-width through var(--hr-weight, 4).
        assert!(
            html_result.contains(r#"stroke="var(--hr-color, currentColor)""#),
            "expected var(--hr-color, currentColor): {html_result}"
        );
        assert!(
            html_result.contains(r#"stroke-width="var(--hr-weight, 4)""#),
            "expected var(--hr-weight, 4) (medium weight): {html_result}"
        );
        // The CSS custom properties are declared on the root <svg>.
        assert!(html_result.contains("--hr-color: currentColor"));
        assert!(html_result.contains("--hr-weight: 4"));
    }

    // ================================================================
    // Phase 3: A4 — CSS-variable strategy flowing through darkmatter
    // ================================================================

    #[test]
    fn test_darkmatter_html_emits_css_variables_for_horizontal_rule() {
        // Default HR (no attributes) should still emit CSS custom properties
        // through the darkmatter HTML pipeline.
        let markdown = "--- { }";
        let md: Markdown = markdown.into();
        let html = md.as_html(Default::default()).unwrap();
        assert!(
            html.contains("--hr-weight:"),
            "expected --hr-weight declaration via pipeline: {html}"
        );
        assert!(
            html.contains("--hr-color:"),
            "expected --hr-color declaration via pipeline: {html}"
        );
        assert!(
            html.contains("--hr-width:"),
            "expected --hr-width declaration via pipeline: {html}"
        );
        assert!(
            html.contains("var(--hr-color,"),
            "expected var(--hr-color, …) via pipeline: {html}"
        );
        assert!(
            html.contains("var(--hr-weight,"),
            "expected var(--hr-weight, …) via pipeline: {html}"
        );
    }

    #[test]
    fn test_darkmatter_html_css_variables_reflect_attributes() {
        // The component-level attributes must show up as matching `--hr-*`
        // declarations on the emitted SVG.
        let markdown = "--- { style: waves, weight: thick, color: \"blue\", width: \"42%\" }";
        let md: Markdown = markdown.into();
        let html = md.as_html(Default::default()).unwrap();
        assert!(
            html.contains("--hr-weight: 8"),
            "thick weight must declare --hr-weight: 8: {html}"
        );
        assert!(
            html.contains("--hr-color: blue"),
            "color attr must declare --hr-color: blue: {html}"
        );
        assert!(
            html.contains("--hr-width: 42%"),
            "width attr must declare --hr-width: 42%: {html}"
        );
    }

    // ================================================================
    // Phase 5: darkmatter integration — B1, B2, B3, B4
    // ================================================================

    #[test]
    fn test_bare_rule_terminal_output_emits_default_dashes() {
        // B4: a bare `---` surfaces as `Event::Rule` (no attributes). The
        // terminal renderer must produce a default dashed rule instead of
        // silently dropping the event through the catch-all arm.
        let markdown = "---\n";
        let md: Markdown = markdown.into();
        let result = md.as_terminal(terminal_text_options()).unwrap();
        assert!(!result.is_empty());
        // Default style is dashes; Unicode mode uses ╌, ASCII fallback uses -.
        assert!(
            result.contains('╌') || result.contains('-'),
            "bare --- should produce default dashes: {result:?}"
        );
    }

    #[test]
    fn test_bare_rule_html_output_emits_default_svg() {
        // B4: bare `---` must emit a default SVG in HTML output, not fall
        // through to the catch-all arm (which would drop the event).
        let markdown = "---\n";
        let md: Markdown = markdown.into();
        let html = md.as_html(Default::default()).unwrap();
        assert!(
            html.contains("<svg "),
            "bare --- should emit an <svg> element: {html}"
        );
        assert!(
            html.contains("--hr-color: currentColor"),
            "bare --- should emit the default currentColor variable: {html}"
        );
    }

    #[test]
    fn test_bare_rule_and_attribute_rule_both_emit_svg() {
        // A document with one bare rule and one attributed rule should
        // produce two SVGs, confirming both code paths (Event::Rule and
        // InlineEvent::HorizontalRule) route through the HTML renderer.
        let markdown = "First\n\n---\n\n--- { style: waves }\n";
        let md: Markdown = markdown.into();
        let html = md.as_html(Default::default()).unwrap();
        let svg_count = html.matches("<svg ").count();
        assert_eq!(
            svg_count, 2,
            "expected two <svg> elements (one per rule path): {html}"
        );
    }

    #[test]
    fn test_terminal_options_width_flows_through_to_rule() {
        // B2: the outer `Terminal` (derived from TerminalOptions.max_width)
        // must reach the HR renderer instead of the old `Terminal::new()`
        // that re-detects capabilities. We assert by forcing a narrow width
        // and confirming the rendered rule body does not exceed it.
        let markdown = "--- { style: dashes, alignment: full }\n";
        let md: Markdown = markdown.into();
        let mut options = terminal_text_options();
        options.max_width = Some(20);
        let result = md.as_terminal(options).unwrap();

        // Find the line that contains the rule body (Unicode ╌ or ASCII -).
        let rule_line = result
            .lines()
            .find(|l| l.contains('╌') || (l.contains('-') && !l.is_empty()))
            .expect("expected a rule line in output");
        // Count visible chars (ignoring ANSI escapes). Every rule char is
        // one column wide.
        let visible = strip_ansi_escape_sequences(rule_line).chars().count();
        assert!(
            visible <= 20,
            "rule line exceeded configured max_width 20: {visible} chars in {rule_line:?}"
        );
    }

    #[test]
    fn test_terminal_options_color_depth_none_disables_ansi() {
        // B2 corollary: when TerminalOptions pin color_depth to None, the HR
        // renderer must honor that (via the shared outer Terminal) and emit
        // no ANSI escapes regardless of the `color` attribute.
        use darkmatter::markdown::output::terminal::ColorDepth;
        let markdown = "--- { color: red }\n";
        let md: Markdown = markdown.into();
        let mut options = TerminalOptions::default();
        options.color_depth = Some(ColorDepth::None);
        let result = md.as_terminal(options).unwrap();
        assert!(
            !result.contains("\x1b["),
            "expected no ANSI escapes with color_depth=None: {result:?}"
        );
    }

    #[test]
    #[serial_test::serial(locale_env)]
    fn test_custom_weight_thick_differs_from_thin() {
        // Phase 5 sanity check: thick and thin must produce different bytes
        // in terminal output now that weight is honored.
        //
        // Only the Unicode tier has a heavy variant (`╍` vs `╌`); the ASCII
        // fallback is weight-insensitive by contract. Pin `LC_ALL` so a host
        // running under `LANG=C` (common on Linux) still exercises the tier
        // this assertion is about. `env_says_utf8` reads `LC_ALL` first, so
        // this one variable outranks any ambient `LC_CTYPE`/`LANG`.
        let _locale = EnvGuard::set_safe("LC_ALL", "en_US.UTF-8");

        let thin: Markdown = "--- { style: dashes, weight: thin }\n".into();
        let thick: Markdown = "--- { style: dashes, weight: thick }\n".into();
        let thin_out = thin.as_terminal(terminal_text_options()).unwrap();
        let thick_out = thick.as_terminal(terminal_text_options()).unwrap();
        assert_ne!(
            thin_out, thick_out,
            "thick rule must differ from thin rule in terminal output"
        );
    }

    #[test]
    fn test_invalid_style_falls_back_to_default() {
        // B1: unknown `style` values must fall back to the default (dashes)
        // without panicking. The builder emits a tracing::warn! but the
        // renderer continues.
        let markdown = "--- { style: bogus }\n";
        let md: Markdown = markdown.into();
        let result = md.as_terminal(terminal_text_options()).unwrap();
        assert!(!result.is_empty());
        assert!(
            result.contains('╌') || result.contains('-'),
            "unknown style should render default dashes: {result:?}"
        );
    }

    #[test]
    fn test_invalid_attribute_values_render_defaults_in_html() {
        // B1 (HTML): unknown enum values must still produce a valid SVG.
        let markdown = "--- { style: bogus, weight: zzz, alignment: diagonal }\n";
        let md: Markdown = markdown.into();
        let html = md.as_html(Default::default()).unwrap();
        assert!(html.contains("<svg "), "expected an <svg>: {html}");
        // Default weight is Medium (4).
        assert!(
            html.contains("--hr-weight: 4"),
            "unknown weight should fall back to default (4): {html}"
        );
    }

    #[test]
    fn test_layout_bottom_margin_honored_for_horizontal_rule() {
        // B3: the HR output path honors the rule's layout margins. With the
        // default `Margin::None`, exactly one trailing blank line sits
        // between the rule and the next block — i.e., rule line + 1 blank
        // line + next block (this is the explicit replacement for the old
        // hardcoded "\n\n"). There must NOT be two blank lines.
        let markdown = "--- { style: dashes }\n\nAfter\n";
        let md: Markdown = markdown.into();
        let result = md.as_terminal(terminal_text_options()).unwrap();

        // Find the rule line and assert the following two lines are "blank
        // then content" (one blank line), not "blank then blank then content".
        let lines: Vec<&str> = result.lines().collect();
        let rule_idx = lines
            .iter()
            .position(|l| l.contains('╌') || l.contains('-'))
            .expect("expected a rule line");
        // Immediately after the rule line, we expect at most one blank
        // line before the next non-blank line.
        let mut blank_run = 0usize;
        for line in &lines[rule_idx + 1..] {
            if line.trim().is_empty() {
                blank_run += 1;
            } else {
                break;
            }
        }
        assert!(
            blank_run <= 1,
            "expected at most 1 blank line between rule and following content, got {blank_run}: {result:?}"
        );
    }

    // Utility: strip ANSI escape sequences so we can measure visible width.
    fn strip_ansi_escape_sequences(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                if chars.next() == Some('[') {
                    for cc in chars.by_ref() {
                        if ('\x40'..='\x7e').contains(&cc) {
                            break;
                        }
                    }
                }
                continue;
            }
            out.push(c);
        }
        out
    }

    // ================================================================
    // Phase 6: C3 — HTML pipeline coverage
    // ================================================================

    #[test]
    fn test_html_multiple_hrs_each_emits_own_svg() {
        // Three attributed horizontal rules must each produce their own
        // `<svg>` element in the HTML output. This is a stronger assertion
        // than the pre-existing "contains stroke-width" test — it counts
        // the SVGs so regressions that coalesce or drop rules are caught.
        let markdown = "\
            --- { style: dashes }\n\
            \n\
            Some content\n\
            \n\
            *** { style: waves }\n\
            \n\
            More\n\
            \n\
            ___ { style: dots }\n";
        let md: darkmatter::markdown::Markdown = markdown.into();
        let html = md.as_html(Default::default()).unwrap();
        let svg_count = html.matches("<svg ").count();
        assert_eq!(
            svg_count, 3,
            "expected three <svg> elements (one per HR): {html}"
        );
    }

    #[test]
    fn test_html_invalid_style_and_weight_render_default_svg() {
        // Unknown `style` and `weight` values must still produce a valid
        // SVG with the defaults applied (B1 + C3). The HR renders — no
        // panic, no dropped element, and the default weight (4 = medium)
        // shows up in the declared CSS custom property.
        let markdown = "--- { style: completely-bogus, weight: ultra }\n";
        let md: darkmatter::markdown::Markdown = markdown.into();
        let html = md.as_html(Default::default()).unwrap();
        assert!(html.contains("<svg "), "expected an <svg>: {html}");
        assert!(
            html.contains("--hr-weight: 4"),
            "unknown weight should fall back to default 4: {html}"
        );
    }

    #[test]
    fn test_html_inline_variables_empty_preserves_var_tokens() {
        // When `hr_css_variables` is empty (the default), the SVG output
        // must keep its `var(--hr-*, …)` expressions so page-level CSS
        // can control the appearance. This is the inverse of the
        // "override flows through" test.
        use darkmatter::markdown::output::HtmlOptions;

        let markdown = "--- { style: dashes }\n";
        let md: darkmatter::markdown::Markdown = markdown.into();
        let options = HtmlOptions::default();
        assert!(
            options.hr_css_variables.is_empty(),
            "default hr_css_variables must be empty"
        );
        let html = md.as_html(options).unwrap();

        assert!(
            html.contains("var(--hr-color,"),
            "default render should keep var(--hr-color, …): {html}"
        );
        assert!(
            html.contains("var(--hr-weight,"),
            "default render should keep var(--hr-weight, …): {html}"
        );
    }

    #[test]
    fn test_horizontal_rule_edge_cases() {
        // Test with various edge cases
        let test_cases = vec![
            "--- { style: dashes }",
            "*** { alignment: centered }",
            "___ { weight: thin }",
            "--- { width: \"100%\" }",
            "*** { color: \"blue\" }",
        ];

        for markdown in test_cases {
            let md: Markdown = markdown.into();
            let terminal_result = md.as_terminal(terminal_text_options()).unwrap();
            let html_result = md.as_html(Default::default()).unwrap();

            assert!(!terminal_result.is_empty());
            assert!(!html_result.is_empty());
        }
    }

    // ================================================================
    // Phase 2: B3 / C2 — unquoted scalar frontmatter must preserve siblings
    // ================================================================

    #[test]
    fn test_hr_frontmatter_non_mapping_does_not_panic() {
        // `hr: 42` is neither a mapping nor absent — the output pipeline
        // must fall back to component defaults without panicking. (The
        // accompanying warn log assertion lives inside
        // `darkmatter::markdown::block::hr_builder`'s own unit tests so the
        // crate-scoped `tracing_test` subscriber captures the event.)
        let markdown = "---\nhr: 42\n---\n\n---\n";
        let md: Markdown = markdown.into();
        let result = md.as_terminal(TerminalOptions::default()).unwrap();
        assert!(
            !result.is_empty(),
            "non-mapping hr frontmatter must still render a default rule"
        );
    }


    #[test]
    fn test_terminal_image_mode_never_disables_hr_image_tier() {
        let markdown = "--- { style: waves, alignment: centered, color: \"red\" }";
        let md: Markdown = markdown.into();
        let mut options = TerminalOptions::default();
        options.image_mode = TerminalImageMode::Never;
        options.max_width = Some(40);
        options.color_depth = Some(ColorDepth::TrueColor);

        let output = md.as_terminal(options).unwrap();

        assert!(
            !output.contains("\x1b_G"),
            "TerminalImageMode::Never must not emit Kitty image escapes: {output:?}"
        );
        assert!(
            !output.contains("\x1b]1337;File="),
            "TerminalImageMode::Never must not emit iTerm image escapes: {output:?}"
        );
        assert!(
            output.contains('≋') || output.contains('~'),
            "TerminalImageMode::Never should fall back to text HR output: {output:?}"
        );
    }

    #[test]
    fn test_terminal_image_mode_force_enables_hr_image_tier() {
        let markdown = "---\n\nSome content after";
        let md: Markdown = markdown.into();
        let mut options = TerminalOptions::default();
        options.image_mode = TerminalImageMode::Force;
        options.max_width = Some(80);
        options.color_depth = Some(ColorDepth::TrueColor);

        let output = md.as_terminal(options).unwrap();

        assert!(
            output.contains("\x1b_G"),
            "TerminalImageMode::Force must emit Kitty image escapes for HR: {output:?}"
        );
    }

    // ================================================================
    // Page-level HR defaults must reach bare `---` rules through the
    // DarkmatterPage render path. These exercise the full `style.hr.*`
    // frontmatter → page → render-tree path the CLI render pipeline uses.
    // ================================================================

    /// Parses `md`'s `style.hr.*` frontmatter and applies it onto a default page.
    fn page_with_frontmatter_hr(md: &Markdown) -> DarkmatterPage {
        let (style, _warnings) =
            from_frontmatter(md.frontmatter()).expect("parse style frontmatter");
        let term = Terminal::new_optimistic(80);
        apply_hr_style(DarkmatterPage::new(&term), &style, HrStyleOverrides::default())
            .expect("apply hr style")
    }

    #[test]
    fn bare_rule_uses_style_hr_frontmatter_defaults_in_html() {
        // `style.hr.*` defaults must style a bare `---` on the browser path.
        let markdown =
            "---\nstyle:\n  hr:\n    kind: waves\n    weight: thick\n    width: \"50%\"\n---\n\n---\n";
        let md: Markdown = markdown.into();
        let html = page_with_frontmatter_hr(&md).render_to_browser(&md).unwrap();
        assert!(
            html.contains(r#"width="50%""#),
            "frontmatter width must apply to the bare rule: {html}"
        );
        assert!(html.contains("--hr-weight: 8"), "thick default ⇒ 8px: {html}");
        assert!(html.contains("<path"), "waves default ⇒ <path> svg: {html}");
    }

    #[test]
    fn top_level_hr_frontmatter_is_ignored_for_bare_rule_defaults_in_html() {
        let markdown =
            "---\nhr:\n  kind: waves\n  weight: thick\n  width: \"50%\"\n---\n\n---\n";
        let md: Markdown = markdown.into();
        let direct_html = md.as_html(Default::default()).unwrap();
        let page_html = page_with_frontmatter_hr(&md).render_to_browser(&md).unwrap();

        for html in [direct_html, page_html] {
            assert!(html.contains(r#"width="100%""#), "{html}");
            assert!(!html.contains(r#"width="50%""#), "{html}");
            assert!(!html.contains("--hr-weight: 8"), "{html}");
            assert!(!html.contains("<path"), "{html}");
        }
    }

    #[test]
    fn rule_attributes_override_frontmatter_defaults_partially_in_html() {
        // Inline rule attributes win per-property; unset properties fall back
        // to the `style.hr.*` defaults.
        let markdown =
            "---\nstyle:\n  hr:\n    kind: waves\n    weight: thick\n    width: \"80%\"\n---\n\n--- { width: \"25%\" }\n";
        let md: Markdown = markdown.into();
        let html = page_with_frontmatter_hr(&md).render_to_browser(&md).unwrap();
        assert!(
            html.contains(r#"width="25%""#),
            "inline width must win over the frontmatter default: {html}"
        );
        assert!(
            html.contains("--hr-weight: 8"),
            "weight must fall back to the thick default: {html}"
        );
        assert!(
            html.contains("<path"),
            "kind must fall back to the waves default: {html}"
        );
    }

    #[test]
    fn bare_rule_uses_style_hr_frontmatter_defaults_in_terminal() {
        // `style.hr.*` defaults must style a bare `---` on the terminal path.
        let markdown = "---\nstyle:\n  hr:\n    kind: dots\n---\n\n---\n";
        let md: Markdown = markdown.into();
        let page = page_with_frontmatter_hr(&md).with_image_mode(TerminalImageMode::Never);
        let out = page.render(&md).unwrap();
        // The default dashed rule uses `╌`/`-`; a dots default switches the
        // glyph to `·` (or the ASCII `.` fallback).
        assert!(
            out.contains('·') || out.contains('.'),
            "bare rule must adopt the `dots` frontmatter default: {out:?}"
        );
    }

}
