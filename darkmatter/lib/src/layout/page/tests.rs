
    use super::*;
    use serial_test::serial;

    fn align_policy(alignment: renderable::layout::Alignment) -> ComponentPolicy {
        let mut policy = ComponentPolicy::default();
        policy.layout.alignment = alignment;
        policy
    }

    fn pad_policy(n: u16) -> ComponentPolicy {
        let mut policy = ComponentPolicy::default();
        policy.layout.padding = renderable::layout::Edges::x(renderable::layout::Length::ch(u32::from(n)));
        policy
    }

    fn max_width_policy(n: u16) -> ComponentPolicy {
        let mut policy = ComponentPolicy::default();
        policy.layout.max_width = Some(renderable::layout::TargetValue::universal(renderable::layout::Length::ch(u32::from(n))));
        policy
    }

    #[allow(dead_code)]
    fn explicit_width_policy(n: u16) -> ComponentPolicy {
        let mut policy = ComponentPolicy::default();
        policy.layout.width = renderable::layout::Width::Fixed(renderable::layout::TargetValue::universal(renderable::layout::Length::ch(u32::from(n))));
        policy
    }

    fn indent_policy(n: u16) -> ComponentPolicy {
        let mut policy = ComponentPolicy::default();
        policy.layout.padding = renderable::layout::Edges {
            left: renderable::layout::TargetValue::universal(renderable::layout::Length::ch(u32::from(n))),
            ..renderable::layout::Edges::default()
        };
        policy
    }

    fn left_margin_policy(n: u16) -> ComponentPolicy {
        let mut policy = ComponentPolicy::default();
        policy.layout.margin.left = renderable::layout::TargetValue::universal(renderable::layout::Length::ch(u32::from(n)));
        policy
    }

    fn edge_ch(tv: &renderable::layout::TargetValue<renderable::layout::Length>) -> u16 {
        match tv {
            renderable::layout::TargetValue::Universal(renderable::layout::Length::Ch(n)) => u16::try_from(*n).unwrap_or(u16::MAX),
            _ => 0,
        }
    }

    fn page() -> DarkmatterPage {
        let term = Terminal::new_optimistic(120);
        DarkmatterPage::new(&term)
    }

    #[test]
    fn defaults_match_spec() {
        let page = page();
        assert_eq!(edge_ch(&page.page_margin().top), 0);
        assert_eq!(edge_ch(&page.page_margin().right), 0);
        assert_eq!(edge_ch(&page.page_margin().bottom), 0);
        assert_eq!(edge_ch(&page.page_margin().left), 0);
        assert_eq!(edge_ch(&page.page_padding().top), 0);
        assert_eq!(edge_ch(&page.page_padding().right), 0);
        assert_eq!(edge_ch(&page.page_padding().bottom), 0);
        assert_eq!(edge_ch(&page.page_padding().left), 0);
        assert_eq!(page.page_background(), PageBackground::Transparent);
        assert_eq!(page.max_width(), None);
        assert!(!page.line_numbers());
        assert_eq!(
            page.component_policy(PageComponent::Images).map(|p| p.layout.alignment).unwrap_or_default(),
            renderable::layout::Alignment::Left
        );
        assert!(page.component_policy(PageComponent::CodeBlocks).is_none());
        assert!(page.is_default_layout());
    }

    #[test]
    fn captures_terminal_width() {
        let page = page();
        assert_eq!(page.terminal_width(), 120);
    }

    #[test]
    fn margin_shorthand_then_specific_overrides() {
        let page = page().with_margin(2).with_margin_top(0);
        let m = page.page_margin();
        assert_eq!(edge_ch(&m.top), 0);
        assert_eq!(edge_ch(&m.right), 2);
        assert_eq!(edge_ch(&m.bottom), 2);
        assert_eq!(edge_ch(&m.left), 2);
    }

    #[test]
    fn margin_axis_helpers() {
        let page = page().with_margin_x(3).with_margin_y(1);
        let m = page.page_margin();
        assert_eq!(edge_ch(&m.left), 3);
        assert_eq!(edge_ch(&m.right), 3);
        assert_eq!(edge_ch(&m.top), 1);
        assert_eq!(edge_ch(&m.bottom), 1);
    }

    #[test]
    fn padding_shorthand_then_specific_overrides() {
        let page = page().with_padding(2).with_padding_left(0);
        let p = page.page_padding();
        assert_eq!(edge_ch(&p.top), 2);
        assert_eq!(edge_ch(&p.right), 2);
        assert_eq!(edge_ch(&p.bottom), 2);
        assert_eq!(edge_ch(&p.left), 0);
    }

    #[test]
    fn use_line_numbers_sets_flag() {
        let page = page().use_line_numbers();
        assert!(page.line_numbers());

        let page = page.with_line_numbers(false);
        assert!(!page.line_numbers());
    }

    #[test]
    fn alignment_overrides_per_component() {
        let mut page = page();
        for component in PageComponent::ALL {
            page = page.with_component_policy(component, align_policy(renderable::layout::Alignment::Center));
        }
        let page = page.with_component_policy(PageComponent::Images, align_policy(renderable::layout::Alignment::Left));
        assert_eq!(
            page.component_policy(PageComponent::Images).map(|p| p.layout.alignment).unwrap_or_default(),
            renderable::layout::Alignment::Left
        );
        assert_eq!(
            page.component_policy(PageComponent::Tables).map(|p| p.layout.alignment).unwrap_or_default(),
            renderable::layout::Alignment::Center
        );
    }

    #[test]
    fn fill_overrides_per_component() {
        let mut page = page();
        for component in PageComponent::ALL {
            page = page.with_component_policy(component, pad_policy(2));
        }
        // Full is the default — remove the CodeBlocks override to restore default.
        let page = page.with_component_policy(PageComponent::CodeBlocks, ComponentPolicy::default());
        assert!(page.component_policy(PageComponent::CodeBlocks).map(|p| p.layout.padding == renderable::layout::Edges::default()).unwrap_or(true));
        assert_eq!(
            edge_ch(&page.component_policy(PageComponent::Tables).unwrap().layout.padding.left),
            2
        );
    }

    #[test]
    fn list_left_margin_accessor() {
        let page = page().with_component_policy(PageComponent::Ul, left_margin_policy(4));
        assert_eq!(
            edge_ch(&page.component_policy(PageComponent::Ul).unwrap().layout.margin.left),
            4
        );
        assert!(page.component_policy(PageComponent::Ol).is_none());
    }

    #[test]
    fn validate_horizontal_space_rejects_overflow() {
        let term = Terminal::new_optimistic(10);
        let page = DarkmatterPage::new(&term)
            .with_margin_x(5)
            .with_padding_x(1);
        let err = page.validate_horizontal_space().unwrap_err();
        assert_eq!(
            err,
            PageRenderError::MarginsExceedTerminalWidth {
                terminal_width: 10,
                required: 12,
            }
        );
    }

    #[test]
    fn validate_horizontal_space_allows_under_width() {
        let page = page().with_margin_x(4).with_padding_x(2);
        page.validate_horizontal_space().unwrap();
    }

    #[test]
    fn validate_max_width_rejects_zero() {
        let page = page().with_max_width(0);
        assert_eq!(
            page.validate_max_width().unwrap_err(),
            PageRenderError::MaxWidthZero
        );
    }

    #[test]
    fn validate_max_width_accepts_unset() {
        page().validate_max_width().unwrap();
    }

    #[test]
    fn validate_max_width_accepts_positive() {
        page().with_max_width(80).validate_max_width().unwrap();
    }

    #[test]
    fn validate_runs_in_order() {
        let term = Terminal::new_optimistic(10);
        let page = DarkmatterPage::new(&term)
            .with_margin_x(5)
            .with_padding_x(1)
            .with_max_width(0);
        // horizontal-space check fires first
        assert!(matches!(
            page.validate().unwrap_err(),
            PageRenderError::MarginsExceedTerminalWidth { .. }
        ));
    }

    #[test]
    fn terminal_options_passthrough_overrides_after_replace() {
        let custom = TerminalOptions {
            image_mode: TerminalImageMode::Never,
            ..TerminalOptions::default()
        };
        let page = page()
            .with_terminal_options(custom)
            .with_image_mode(TerminalImageMode::Force);
        assert_eq!(page.terminal_options().image_mode, TerminalImageMode::Force);
    }

    #[test]
    fn captures_terminal_color_mode() {
        let page = page();
        // Optimistic terminal default color_mode value is exposed.
        let _mode = *page.terminal_color_mode();
    }

    // ---------- Phase 2: render tests ----------

    #[test]
    #[serial]
    fn zero_config_render_ignores_captured_terminal_width() {
        // Construct a DarkmatterPage from a Terminal whose captured width
        // differs from TerminalOptions::default() auto-detection. The page
        // must NOT leak that captured width into component width resolution;
        // output must remain byte-for-byte identical to the default
        // `as_terminal()` render. Without the `is_default_layout()` short-circuit in
        // `render`, image/list/blockquote/table/code component paths would
        // resolve widths against the captured Terminal width and diverge.
        for width in [40u32, 100, 200] {
            let term = Terminal::new_optimistic(width);
            let page = DarkmatterPage::new(&term);
            let md: Markdown = "# Heading\n\n- List item\n\n> Quoted prose\n\n```rust\nfn main() {}\n```\n\n| A | B |\n| - | - |\n| 1 | 2 |\n".into();

            let page_out = page.render(&md).unwrap();
            let direct_out = md.as_terminal(TerminalOptions::default()).unwrap();

            assert_eq!(
                page_out, direct_out,
                "zero-config render with captured_width={width} must equal the default as_terminal render",
            );
        }
    }

    /// Phase 3.2: a default-layout `DarkmatterPage` browser render that
    /// captures a Terminal width different from the ambient detection must
    /// still produce a non-wrapped body — the captured width must not leak
    /// into the page wrapper, and a `with_page_bg_color` flag must reach the
    /// page wrapper CSS.
    #[test]
    fn zero_config_browser_render_captures_terminal_width_without_leak() {
        let term = Terminal::new_optimistic(40);
        let page = DarkmatterPage::new(&term);
        let md: Markdown = "# Hello\n\nA paragraph.\n".into();

        let html = page.render_to_browser(&md).unwrap();
        assert!(
            !html.contains("<div class=\"darkmatter-page\""),
            "a default-layout page should not add a page wrapper; got: {html}"
        );
    }

    /// `DarkmatterPage::new` must capture the [`Terminal`]'s color depth so a
    /// page built from `new_optimistic` (hardcoded `TrueColor`) reports that
    /// depth regardless of ambient detection.
    #[test]
    fn new_captures_terminal_color_depth() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        assert_eq!(page.terminal_color_depth(), ColorDepth::TrueColor);
    }

    /// On the decorated layout path, the page must thread its captured color
    /// depth into [`TerminalOptions`] so the render honors the [`Terminal`] it
    /// was constructed with rather than re-detecting from the ambient
    /// environment. Without this, a page built from `new_optimistic` in a
    /// headless `cargo test` env would emit 256-color or no-color SGR even
    /// though the captured terminal reports `TrueColor`.
    ///
    /// The truecolor background SGR sequence (`\x1b[48;2;r;g;bm`) is unique to
    /// 24-bit output — its presence is sufficient evidence that the captured
    /// depth was honored.
    #[test]
    fn decorated_render_honors_captured_color_depth() {
        let term = Terminal::new_optimistic(80);
        let md: Markdown = "```rust\nfn main() {}\n```\n".into();
        let out = DarkmatterPage::new(&term)
            .with_margin_left(2)
            .with_margin_right(2)
            .with_code_theme("dracula")
            .render(&md)
            .unwrap();
        assert!(
            out.contains("\x1b[48;2;"),
            "decorated render with `new_optimistic` must emit truecolor SGR"
        );
    }

    /// An explicit [`Self::with_color_depth`] must override the captured
    /// terminal depth, so callers retain precise control when they want it.
    ///
    /// Pinning [`ColorDepth::None`] is the cleanest discriminator: the
    /// terminal renderer detects it at the top of its pipeline and returns the
    /// raw markdown content (no syntax highlighting, no SGR), so a passing
    /// assertion below proves the explicit value reached the renderer rather
    /// than being silently replaced by the captured `TrueColor`. (Verifying a
    /// downgrade between truecolor and 256-color would also require the
    /// highlighter to honor `color_depth`, which is a separate concern from
    /// this gate's contract.)
    #[test]
    fn with_color_depth_overrides_captured_depth() {
        let term = Terminal::new_optimistic(80);
        let md: Markdown = "```rust\nfn main() {}\n```\n".into();
        let out = DarkmatterPage::new(&term)
            .with_color_depth(ColorDepth::None)
            .with_margin_left(2)
            .with_margin_right(2)
            .with_code_theme("dracula")
            .render(&md)
            .unwrap();
        assert!(
            !out.contains("\x1b["),
            "explicit `with_color_depth(None)` must suppress every SGR; got: {out:?}"
        );
    }

    #[test]
    fn render_with_margin_adds_margin_rows() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .with_margin_top(2)
            .with_margin_bottom(1);
        let md: Markdown = "# Hello\n".into();

        let out = page.render(&md).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        // First two lines should be empty (top margin).
        assert!(
            lines[0].trim().is_empty(),
            "first line should be top margin"
        );
        assert!(
            lines[1].trim().is_empty(),
            "second line should be top margin"
        );
        // Last line should be empty (bottom margin).
        assert!(
            lines.last().unwrap().trim().is_empty(),
            "last line should be bottom margin"
        );
    }

    #[test]
    fn render_with_padding_adds_bg_rows() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .with_padding_top(1)
            .with_padding_bottom(1)
            .with_page_background(PageBackground::Subtle);
        let md: Markdown = "# Hello\n".into();

        let out = page.render(&md).unwrap();
        // Should contain ANSI background codes for subtle color.
        assert!(
            out.contains("\x1b[48;2;"),
            "padding rows should have background color"
        );
    }

    /// sRGB relative luminance (0.0 black .. 1.0 white) of an RGB triple.
    fn rel_luminance(r: u8, g: u8, b: u8) -> f32 {
        fn channel(value: u8) -> f32 {
            let value = f32::from(value) / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
    }

    /// RGB of the truecolor background (`\x1b[48;2;r;g;bm`) active where `needle`
    /// is drawn — the last such background set before `needle` in `haystack`.
    fn active_bg_at(haystack: &str, needle: &str) -> (u8, u8, u8) {
        let idx = haystack
            .find(needle)
            .unwrap_or_else(|| panic!("missing {needle:?} in render:\n{haystack:?}"));
        let marker = "\x1b[48;2;";
        let start = haystack[..idx]
            .rfind(marker)
            .unwrap_or_else(|| panic!("no truecolor background before {needle:?}"));
        let rest = &haystack[start + marker.len()..];
        let end = rest.find('m').expect("unterminated SGR");
        let nums: Vec<u8> = rest[..end]
            .split(';')
            .map(|n| n.parse::<u8>().expect("rgb component"))
            .collect();
        (nums[0], nums[1], nums[2])
    }

    /// The Motivating Defect (simplified-rendering spec): in a real DARK terminal
    /// whose option-derived color mode disagrees (the CLI fills
    /// `options.color_mode` from an env-only detector that can resolve Light while
    /// the terminal is Dark), the code panel must still invert against the
    /// *terminal* mode and separate from the dark page surface. Pre-fix the panel
    /// inverts against the option mode and renders dark-on-dark.
    #[test]
    fn code_panel_separates_from_page_surface_in_dark_terminal() {
        let mut term = Terminal::new_optimistic(80);
        term.color_mode = TerminalColorMode::Dark;

        let md: Markdown =
            "# TitleMarker\n\nProseMarker paragraph.\n\n```rust\nfn codemarker() {}\n```\n".into();

        let out = DarkmatterPage::new(&term)
            .with_page_background(PageBackground::Subtle)
            .with_color_mode(ColorMode::Light)
            .render(&md)
            .unwrap();

        let page_bg = active_bg_at(&out, "TitleMarker");
        let panel_bg = active_bg_at(&out, "codemarker");
        let page_lum = rel_luminance(page_bg.0, page_bg.1, page_bg.2);
        let panel_lum = rel_luminance(panel_bg.0, panel_bg.1, panel_bg.2);

        assert!(
            (page_lum - panel_lum).abs() > 0.15,
            "code panel must separate from the dark page surface: \
             page bg {page_bg:?} (lum {page_lum:.3}) vs panel bg {panel_bg:?} (lum {panel_lum:.3})"
        );
    }

    /// The DEFAULT page background is `Transparent` (the `md render` default, with
    /// no `--page-bg`). There is no painted page surface to separate from, but the
    /// code panel must still invert against the *terminal* mode (Decision #4/#9):
    /// a dark terminal yields a light (inverted) panel even when the option mode
    /// disagrees. Pre-fix the panel inverts against the option mode.
    #[test]
    fn code_panel_inverts_against_terminal_not_option_in_transparent_default() {
        let mut term = Terminal::new_optimistic(80);
        term.color_mode = TerminalColorMode::Dark;

        let md: Markdown = "```rust\nfn codemarker() {}\n```\n".into();

        let out = DarkmatterPage::new(&term)
            .with_margin_left(1)
            .with_color_mode(ColorMode::Light)
            .render(&md)
            .unwrap();

        let panel_bg = active_bg_at(&out, "codemarker");
        let lum = rel_luminance(panel_bg.0, panel_bg.1, panel_bg.2);
        assert!(
            lum > 0.5,
            "a dark terminal must invert the code panel to a light theme regardless \
             of the option mode: panel bg {panel_bg:?} (lum {lum:.3})"
        );
    }

    /// Symmetric case: a real LIGHT terminal with a disagreeing option mode. The
    /// panel must invert against the terminal (Light) and separate from the light
    /// page surface.
    #[test]
    fn code_panel_separates_from_page_surface_in_light_terminal() {
        let mut term = Terminal::new_optimistic(80);
        term.color_mode = TerminalColorMode::Light;

        let md: Markdown =
            "# TitleMarker\n\nProseMarker paragraph.\n\n```rust\nfn codemarker() {}\n```\n".into();

        let out = DarkmatterPage::new(&term)
            .with_page_background(PageBackground::Subtle)
            .with_color_mode(ColorMode::Dark)
            .render(&md)
            .unwrap();

        let page_bg = active_bg_at(&out, "TitleMarker");
        let panel_bg = active_bg_at(&out, "codemarker");
        let page_lum = rel_luminance(page_bg.0, page_bg.1, page_bg.2);
        let panel_lum = rel_luminance(panel_bg.0, panel_bg.1, panel_bg.2);

        assert!(
            (page_lum - panel_lum).abs() > 0.15,
            "code panel must separate from the light page surface: \
             page bg {page_bg:?} (lum {page_lum:.3}) vs panel bg {panel_bg:?} (lum {panel_lum:.3})"
        );
    }

    /// Phase 2 (centralize theme resolution): a single `Terminal` is the
    /// source of truth for *both* the page surface and the nested code-block
    /// panel. Construct a dark `Terminal`, build a `DarkmatterPage` from it,
    /// render a fenced code block, and assert the resolved panel mode is
    /// `Light` (the dark terminal's inversion). The page path no longer
    /// threads a separate env-derived `options.color_mode` through to the
    /// code renderer — only the captured terminal's `color_mode()` feeds
    /// the resolution.
    #[test]
    fn dark_terminal_inverts_to_light_panel_via_captured_terminal() {
        let mut term = Terminal::new_optimistic(80);
        term.color_mode = TerminalColorMode::Dark;

        let md: Markdown = "```rust\nfn codemarker() {}\n```\n".into();

        // No `with_color_mode` override: the captured terminal's mode is
        // the only source feeding the layout context and the code renderer.
        let out = DarkmatterPage::new(&term)
            .with_margin_left(1)
            .render(&md)
            .unwrap();
        let panel_bg = active_bg_at(&out, "codemarker");
        let lum = rel_luminance(panel_bg.0, panel_bg.1, panel_bg.2);
        assert!(
            lum > 0.5,
            "a dark terminal captured by DarkmatterPage must invert the code \
             panel to a light theme: panel bg {panel_bg:?} (lum {lum:.3})"
        );

        // Sanity: a light terminal yields a dark panel through the same
        // path. The invariant is symmetric.
        let mut term_light = Terminal::new_optimistic(80);
        term_light.color_mode = TerminalColorMode::Light;
        let out_light = DarkmatterPage::new(&term_light)
            .with_margin_left(1)
            .render(&md)
            .unwrap();
        let panel_bg_light = active_bg_at(&out_light, "codemarker");
        let lum_light = rel_luminance(panel_bg_light.0, panel_bg_light.1, panel_bg_light.2);
        assert!(
            lum_light < 0.5,
            "a light terminal captured by DarkmatterPage must invert the code \
             panel to a dark theme: panel bg {panel_bg_light:?} (lum {lum_light:.3})"
        );
    }

    /// Page-frame boundary (closeout Option A): the page frame is independent of
    /// component policy entirely — neither its presence nor its content. Two
    /// pages with identical frame geometry but different component-policy
    /// *content*, rendering a document whose nodes none of those policies match,
    /// must produce byte-identical output: the frame chrome cannot vary with
    /// which component a policy targets or what color it sets.
    #[test]
    fn page_frame_chrome_ignores_component_policy_content() {
        use renderable::color::{Color, Tailwind};
        use renderable::style::PaintColor;

        let term = Terminal::new_optimistic(80);
        // A document with no table and no block quote: neither policy below has
        // a node to bake onto, so any output difference could only come from the
        // frame inspecting policy content — which it must not do.
        let md: Markdown = "# Title\n\nA plain paragraph with no components.\n".into();

        let page_a = DarkmatterPage::new(&term)
            .with_margin_top(2)
            .with_margin_left(3)
            .with_component_color(
                PageComponent::Tables,
                PaintColor::new(Color::Tailwind(Tailwind::Red500)),
            );
        let page_b = DarkmatterPage::new(&term)
            .with_margin_top(2)
            .with_margin_left(3)
            .with_component_bg_color(
                PageComponent::BlockQuotes,
                PaintColor::new(Color::Tailwind(Tailwind::Blue500)),
            );

        let out_a = page_a.render(&md).unwrap();
        let out_b = page_b.render(&md).unwrap();
        assert_eq!(
            out_a, out_b,
            "page-frame output must be independent of component-policy content",
        );
    }

    /// Review-2 finding 2: the page-frame width cap must key off frame geometry
    /// alone, never component-policy presence. With the captured terminal wider
    /// than the ambient auto-detected width and a document line long enough to
    /// wrap at the ambient width, an *unmatched* component policy on an
    /// otherwise zero-geometry page must produce byte-identical output to a
    /// no-policy page: both render the content box at the ambient width.
    ///
    /// This is discriminating where the 80==80 parity test was not: before the
    /// fix, the unmatched policy made the page non-default, capping `max_width`
    /// to the captured 200-wide terminal so the long line would *not* wrap,
    /// while the no-policy page wrapped at the ambient width — the two diverged.
    #[test]
    fn terminal_unmatched_policy_does_not_cap_width_to_captured_terminal() {
        use renderable::color::{Color, Tailwind};
        use renderable::style::PaintColor;

        // Captured terminal far wider than the ambient (~80) auto-detect, with no
        // frame geometry: width must stay ambient-driven for both pages.
        let term = Terminal::new_optimistic(200);
        // A long single-paragraph line that wraps at ~80 columns but fits on one
        // line at 200. No table, so the Tables policy is unmatched.
        let md: Markdown = "The quick brown fox jumps over the lazy dog, and then the \
            quick brown fox jumps over the lazy dog once more for good wrapping measure."
            .into();

        let no_policy = DarkmatterPage::new(&term).render(&md).unwrap();
        let unmatched = DarkmatterPage::new(&term)
            .with_component_color(
                PageComponent::Tables,
                PaintColor::new(Color::Tailwind(Tailwind::Red500)),
            )
            .render(&md)
            .unwrap();

        assert_eq!(
            no_policy, unmatched,
            "an unmatched policy must not cap the content box to the captured terminal width",
        );
        // Guard the test's own premise: the no-policy render wrapped at the
        // ambient width (more than one line), so a 200-wide cap *would* have
        // changed it — proving this test can discriminate the regression.
        assert!(
            no_policy.lines().filter(|l| !l.trim().is_empty()).count() > 1,
            "test premise: the long line must wrap at the ambient width; got:\n{no_policy}",
        );
    }

    /// Review-4 finding 1: capability selection (renderer-wide color depth) must
    /// be independent of *unmatched* component-policy presence.
    ///
    /// The color depth is pinned explicitly so the parity holds regardless of the
    /// harness's ambient detection — the test must pass even under ambient
    /// no-color. A fenced code block's syntax highlighting is color-bearing and
    /// responds to the depth (no SGR at `None`, truecolor SGR at `TrueColor`), so
    /// it exposes any capability difference between the two pages.
    ///
    /// Before the fix, an unmatched policy made the page non-default, routing the
    /// render through the optimistic pre-render terminal (TrueColor) and so
    /// changing the colored rendering of unrelated content versus the no-policy
    /// page. Both depths must now produce byte-identical output with and without
    /// the unmatched policy.
    #[test]
    fn terminal_unmatched_policy_does_not_flip_color_depth_for_unrelated_content() {
        use renderable::color::{Color, Tailwind};
        use renderable::style::PaintColor;

        let term = Terminal::new_optimistic(80);
        // A fenced code block whose syntax highlighting is color-bearing. No
        // table, so the Tables policy is unmatched and bakes nothing.
        let md: Markdown = "```rust\nfn main() { let x = 1; }\n```\n".into();

        let render_pair = |depth: ColorDepth| {
            let no_policy = DarkmatterPage::new(&term)
                .with_color_depth(depth)
                .render(&md)
                .unwrap();
            let unmatched = DarkmatterPage::new(&term)
                .with_color_depth(depth)
                .with_component_color(
                    PageComponent::Tables,
                    PaintColor::new(Color::Tailwind(Tailwind::Red500)),
                )
                .render(&md)
                .unwrap();
            (no_policy, unmatched)
        };

        // No-color depth: neither page may emit foreground SGR, and the unmatched
        // page must stay byte-identical. (Before the fix the unmatched policy
        // re-introduced TrueColor here via the optimistic terminal.)
        let (no_policy_none, unmatched_none) = render_pair(ColorDepth::None);
        assert_eq!(
            no_policy_none, unmatched_none,
            "an unmatched policy must not flip renderer-wide color depth",
        );
        assert!(
            !no_policy_none.contains("\x1b[38"),
            "no-color depth must strip foreground SGR; got: {no_policy_none:?}",
        );

        // TrueColor depth: the highlighted code must be byte-identical with and
        // without the unmatched policy — the policy changes no capability.
        let (no_policy_true, unmatched_true) = render_pair(ColorDepth::TrueColor);
        assert_eq!(
            no_policy_true, unmatched_true,
            "an unmatched policy must not change colored rendering of unrelated content",
        );
        assert!(
            no_policy_true.contains("\x1b[38;2;"),
            "test premise: TrueColor depth must emit truecolor SGR; got: {no_policy_true:?}",
        );
    }

    /// Counts the renderer-wide capability signals in a terminal render: how many
    /// truecolor, 256-color, 3/4-bit foreground SGRs, and OSC8 hyperlink openers
    /// it carries. Layout shifts (e.g. a centered table's leading spaces) add no
    /// escape sequences, so two renders that differ only in matched layout share
    /// this signature iff they share a capability profile.
    fn capability_signature(s: &str) -> (usize, usize, usize, usize) {
        let truecolor = s.matches("\x1b[38;2;").count();
        let palette = s.matches("\x1b[38;5;").count();
        let osc8 = s.matches("\x1b]8;;").count();
        let basic: usize = (0u8..8)
            .map(|n| {
                s.matches(&format!("\x1b[3{n}m")).count()
                    + s.matches(&format!("\x1b[9{n}m")).count()
            })
            .sum();
        (truecolor, palette, osc8, basic)
    }

    /// Review-5 finding 1 (High): a *matched* layout-only component policy must
    /// not change the renderer-wide capability profile (color depth, OSC8) of
    /// unrelated content. Centering a table is layout-only — it bakes no color —
    /// so the fenced code block's highlight colors and the link's OSC8 behavior
    /// must be identical with and without the policy.
    ///
    /// Before the fix a matched layout policy set `applies = true`, routing the
    /// whole document through the optimistic (TrueColor + OSC8) profile, so
    /// unrelated content gained colors and hyperlinks the ambient terminal never
    /// advertised. The capability *signature* ignores the table's centering
    /// whitespace, so it isolates the capability change from the layout change.
    #[test]
    fn terminal_matched_layout_policy_does_not_change_unrelated_capabilities() {
        let term = Terminal::new_optimistic(120);
        // A table (the matched, layout-only policy target) plus unrelated
        // capability-bearing content: a color-highlighted code block and an OSC8
        // hyperlink.
        let md: Markdown = "| A | B |\n|---|---|\n| 1 | 2 |\n\n\
            ```rust\nfn main() { let x = 1; }\n```\n\n\
            [link](https://example.com)\n"
            .into();

        let no_policy = DarkmatterPage::new(&term).render(&md).unwrap();
        let mut matched_policy = max_width_policy(40);
        matched_policy.layout.alignment = renderable::layout::Alignment::Center;
        let matched = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::Tables, matched_policy)
            .render(&md)
            .unwrap();

        // Premise: the policy actually matched the table — capping its width
        // and centering it shifts the table rows, so the two renders are not
        // byte-identical. (A no-op policy would make the capability comparison
        // below vacuous.)
        assert_ne!(
            no_policy, matched,
            "test premise: the layout policy must match the table and change its rendering",
        );

        // Premise: the optimistic profile — reached only by deliberate geometry —
        // *does* carry truecolor or OSC8 for this fixture, so a regression that
        // wrongly selected it for the matched layout policy would be observable.
        let optimistic = DarkmatterPage::new(&term).with_margin_left(1).render(&md).unwrap();
        let opt_sig = capability_signature(&optimistic);
        assert!(
            opt_sig.0 > 0 || opt_sig.2 > 0,
            "test premise: the optimistic profile must carry truecolor or OSC8 here; sig={opt_sig:?}",
        );

        // The matched layout-only policy must leave the renderer-wide capability
        // profile of unrelated content unchanged.
        assert_eq!(
            capability_signature(&no_policy),
            capability_signature(&matched),
            "a matched layout-only policy must not change the capability profile \
             (color/OSC8) of unrelated content",
        );
    }

    /// When the page *does* paint construction color and no explicit depth is
    /// pinned, the render honors the captured terminal's color depth — proving
    /// the construction-color probe engages and selects the captured capability.
    ///
    /// The same page color rendered against a captured `None`-depth terminal and
    /// a captured `TrueColor` terminal must diverge: the former strips all color,
    /// the latter keeps it.
    #[test]
    fn terminal_painted_color_engages_captured_color_depth() {
        use renderable::color::{Color, Tailwind};
        use renderable::style::PaintColor;

        let md: Markdown = "```rust\nfn main() { let x = 1; }\n```\n".into();

        let mut none_term = Terminal::new_optimistic(80);
        none_term.color_depth = biscuit_terminal::discovery::detection::ColorDepth::None;
        let painted_none = DarkmatterPage::new(&none_term)
            .with_page_color(PaintColor::new(Color::Tailwind(Tailwind::Red500)))
            .render(&md)
            .unwrap();

        // `new_optimistic` reports TrueColor depth.
        let true_term = Terminal::new_optimistic(80);
        let painted_true = DarkmatterPage::new(&true_term)
            .with_page_color(PaintColor::new(Color::Tailwind(Tailwind::Red500)))
            .render(&md)
            .unwrap();

        assert_ne!(
            painted_none, painted_true,
            "painting construction color must engage the captured terminal depth",
        );
    }

    /// Page-frame boundary (closeout Option A): vertical margin is pure additive
    /// chrome. The frame wraps the folded body; it does not traverse or rewrite
    /// component content. Adding top/bottom margin to an otherwise identical
    /// page must only prepend/append blank rows, leaving the component body
    /// (heading, block quote, list, code) byte-identical.
    #[test]
    fn page_frame_vertical_margin_only_wraps_component_body() {
        let term = Terminal::new_optimistic(80);
        let md: Markdown =
            "# Heading\n\n> A quoted line.\n\n- one\n- two\n\n```rust\nlet x = 1;\n```\n".into();

        // Both pages share the same left margin (so both take the decorated path
        // at the same effective width); only the vertical margin differs.
        let base = DarkmatterPage::new(&term).with_margin_left(3);
        let taller = DarkmatterPage::new(&term)
            .with_margin_left(3)
            .with_margin_top(2)
            .with_margin_bottom(2);

        let base_out = base.render(&md).unwrap();
        let tall_out = taller.render(&md).unwrap();

        // Strip leading/trailing fully-blank rows (the only thing vertical
        // margin adds), then the component body must be identical.
        let core = |s: &str| -> String {
            let lines: Vec<&str> = s.lines().collect();
            let start = lines.iter().position(|l| !l.trim().is_empty());
            let end = lines.iter().rposition(|l| !l.trim().is_empty());
            match (start, end) {
                (Some(a), Some(b)) => lines[a..=b].join("\n"),
                _ => String::new(),
            }
        };
        assert_eq!(
            core(&base_out),
            core(&tall_out),
            "vertical margin must only add blank rows; the component body must be unchanged",
        );
    }

    /// Review-1 finding 2: an *unmatched* component policy — one whose target
    /// component is absent from the document — must not change terminal output
    /// versus no policy at all. Frame decisions (width cap, row decoration) key
    /// off page geometry, never policy presence, so a policy with no node to
    /// bake onto leaves the rendered bytes identical.
    #[test]
    fn terminal_unmatched_component_policy_matches_no_policy() {
        use renderable::color::{Color, Tailwind};
        use renderable::style::PaintColor;

        let term = Terminal::new_optimistic(80);
        // No table in the document: the Tables policy has nothing to bake onto.
        let md: Markdown = "# Title\n\nA plain paragraph, no table here.\n".into();

        let no_policy = DarkmatterPage::new(&term).render(&md).unwrap();
        let unmatched = DarkmatterPage::new(&term)
            .with_component_color(
                PageComponent::Tables,
                PaintColor::new(Color::Tailwind(Tailwind::Red500)),
            )
            .render(&md)
            .unwrap();

        assert_eq!(
            no_policy, unmatched,
            "an unmatched component policy must not change terminal output vs no policy",
        );
    }

    /// Review-1 finding 2: the browser analogue — an unmatched component policy
    /// must produce byte-identical HTML to no policy (no spurious page wrapper,
    /// no per-component CSS), since the unmatched policy bakes onto no node.
    #[test]
    fn browser_unmatched_component_policy_matches_no_policy() {
        use renderable::color::{Color, Tailwind};
        use renderable::style::PaintColor;

        let term = Terminal::new_optimistic(80);
        let md: Markdown = "# Title\n\nA plain paragraph, no table here.\n".into();

        let no_policy = DarkmatterPage::new(&term).render_to_browser(&md).unwrap();
        let unmatched = DarkmatterPage::new(&term)
            .with_component_color(
                PageComponent::Tables,
                PaintColor::new(Color::Tailwind(Tailwind::Red500)),
            )
            .render_to_browser(&md)
            .unwrap();

        assert_eq!(
            no_policy, unmatched,
            "an unmatched component policy must not change browser HTML vs no policy",
        );
    }

    #[test]
    fn renderable_trait_with_markdown() {
        let term = Terminal::new_optimistic(80);
        let md: Markdown = "# TerminalRenderable\n".into();
        let page = DarkmatterPage::new(&term).with_markdown(md);

        let out = TerminalRenderable::render(&page, &term);
        let plain = crate::testing::strip_ansi_codes(&out);
        assert!(
            plain.contains("TerminalRenderable"),
            "TerminalRenderable::render should output markdown content"
        );
    }

    #[test]
    fn renderable_trait_without_markdown_shows_placeholder() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);

        let out = TerminalRenderable::render(&page, &term);
        assert!(
            out.contains("no markdown set"),
            "TerminalRenderable without markdown should show placeholder"
        );
    }

    #[test]
    fn renderable_trait_block_level() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        assert!(TerminalRenderable::is_block_level(&page));
    }

    #[test]
    fn renderable_trait_as_any() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        assert!(
            TerminalRenderable::as_any(&page)
                .downcast_ref::<DarkmatterPage>()
                .is_some()
        );
    }

    #[test]
    fn render_with_max_width_caps_content() {
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term).with_max_width(60);
        let md: Markdown =
            "# Hello\n\nThis is a paragraph that should wrap at the max width.\n".into();

        let out = page.render(&md).unwrap();
        // Verify it renders without error.
        assert!(
            !out.is_empty(),
            "render with max_width should produce output"
        );
    }

    #[test]
    fn render_error_for_max_width_zero() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term).with_max_width(0);
        let md: Markdown = "# Hello\n".into();

        let err = page.render(&md).unwrap_err();
        assert_eq!(err, PageRenderError::MaxWidthZero);
    }

    #[test]
    fn render_error_for_margins_exceed_width() {
        let term = Terminal::new_optimistic(10);
        let page = DarkmatterPage::new(&term)
            .with_margin_x(5)
            .with_padding_x(1);
        let md: Markdown = "# Hello\n".into();

        let err = page.render(&md).unwrap_err();
        assert!(matches!(
            err,
            PageRenderError::MarginsExceedTerminalWidth { .. }
        ));
    }

    // ---------- Phase 3: component layout tests ----------

    #[test]
    fn render_code_block_center_aligned_with_max_fill() {
        let term = Terminal::new_optimistic(80);
        let mut policy = max_width_policy(40);
        policy.layout.alignment = renderable::layout::Alignment::Center;
        let page = DarkmatterPage::new(&term)
            .with_max_width(80)
            .with_component_policy(PageComponent::CodeBlocks, policy);
        let md: Markdown = "```rust\nfn main() {}\n```\n".into();

        let out = page.render(&md).unwrap();
        let plain = crate::testing::strip_ansi_codes(&out);
        // The explicit 80-column page frame makes the component's containing
        // width independent of ambient terminal detection. With Max(40), the
        // code block header renders at 40 cols, then the whole 40-col block is
        // centered in 80 => 20 spaces of alignment padding.
        // The header title is right-aligned within the 40-col block, so the
        // "rust" token is preceded by that padding plus ~34 header spaces.
        let first_line = plain.lines().next().unwrap();
        let leading_spaces = first_line.len() - first_line.trim_start().len();
        assert!(
            leading_spaces >= 50,
            "code block header should be centered with significant left padding, got {} leading spaces: {:?}",
            leading_spaces,
            first_line
        );
        assert!(first_line.contains("rust"));
    }

    #[test]
    fn render_table_right_aligned_with_max_fill() {
        let term = Terminal::new_optimistic(80);
        let mut policy = max_width_policy(30);
        policy.layout.alignment = renderable::layout::Alignment::Right;
        let page = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::Tables, policy);
        let md: Markdown = "| A | B |\n|---|---|\n| 1 | 2 |\n".into();

        let out = page.render(&md).unwrap();
        let plain = crate::testing::strip_ansi_codes(&out);
        // Table rendered at 30 cols, right-aligned in 80 => left pad = 80-30 = 50.
        let table_lines: Vec<&str> = plain
            .lines()
            .filter(|l| l.contains('│') || l.contains('┌') || l.contains('├') || l.contains('└'))
            .collect();
        assert!(!table_lines.is_empty(), "table should render");
        let first = table_lines[0];
        assert!(
            first.starts_with("                                                  "),
            "table should be right-aligned, got: {:?}",
            first
        );
    }

    #[test]
    fn render_code_block_with_max_fill() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::CodeBlocks, max_width_policy(40));
        let md: Markdown = "```rust\nfn main() {}\n```\n".into();

        let out = page.render(&md).unwrap();
        let plain = crate::testing::strip_ansi_codes(&out);
        // Header row should be capped at 40 cols.
        let first_line = plain.lines().next().unwrap();
        assert!(
            first_line.len() <= 40,
            "code block header should be capped to 40 cols, got len={}",
            first_line.len()
        );
    }

    #[test]
    fn render_code_block_with_pad_fill() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .with_max_width(80)
            .with_component_policy(PageComponent::CodeBlocks, pad_policy(4));
        let md: Markdown = "```rust\nfn main() {}\n```\n".into();

        let out = page.render(&md).unwrap();
        let plain = crate::testing::strip_ansi_codes(&out);

        // The explicit 80-column page frame makes the component's containing
        // width independent of ambient terminal detection. The second line is
        // the top padding row, the simplest line to measure because it carries
        // no header text: a full-component-width background fill row whose left
        // edge carries the 4-col Pad padding.
        let padding_row = plain.lines().nth(1).unwrap();
        assert_eq!(
            padding_row.len(),
            80,
            "padding row should span the full component width, got len={}",
            padding_row.len()
        );
        assert!(
            padding_row.starts_with("    "),
            "padding row should start with 4 leading spaces (Pad left padding)"
        );
    }

    #[test]
    fn render_blockquote_with_indent_fill() {
        let term = Terminal::new_optimistic(80);
        let mut policy = indent_policy(10);
        policy.layout.alignment = renderable::layout::Alignment::Left;
        let page = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::BlockQuotes, policy);
        // Long content so the wrap point is observable. Without the active
        // width override, this line would render in a single 80-col span.
        let md: Markdown = "> This is a very long quoted paragraph that should be forced to wrap once the component-specific width override is applied, leaving the remaining text on subsequent lines below.\n".into();

        let out = page.render(&md).unwrap();
        let plain = crate::testing::strip_ansi_codes(&out);
        for (i, line) in plain.lines().enumerate() {
            eprintln!("DEBUG bq line {}: len={} {:?}", i, line.len(), line);
        }
        // Strip the blockquote prefix `▐   ` (4 visible cols) from each line.
        // With Indent(10) at 80 cols, prose wraps at 70 cols. The blockquote
        // prefix consumes 4 cols, so the final line content widths should not
        // exceed 70 visible columns.
        let lines: Vec<String> = plain
            .lines()
            .filter(|l| l.contains('▐'))
            .map(|l| l.trim_end().to_string())
            .collect();
        assert!(
            lines.len() >= 2,
            "blockquote should wrap onto multiple lines under Indent(10); got {} line(s):\n{}",
            lines.len(),
            plain
        );
        let max_len = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        assert!(
            max_len <= 75,
            "blockquote lines should be capped by Indent(10), got max={}:\n{}",
            max_len,
            plain
        );
    }

    #[test]
    fn render_list_with_max_fill() {
        let term = Terminal::new_optimistic(80);
        let mut page = DarkmatterPage::new(&term);
        for component in [PageComponent::Ul, PageComponent::Ol, PageComponent::Li] {
            page = page.with_component_policy(component, max_width_policy(50));
        }
        // Long list item so wrap is observable. Without the active width
        // override, this would render at the page width (80) on a single line.
        let md: Markdown = "- This is an unusually long bullet item that ought to be forced to wrap once Max(50) constrains the list rendering width to fifty columns.\n- Short follow-up.\n".into();

        let out = page.render(&md).unwrap();
        let plain = crate::testing::strip_ansi_codes(&out);
        let lines: Vec<&str> = plain.lines().collect();
        let max_len = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        assert!(
            max_len <= 50,
            "list lines should be capped to 50 cols, got max={}:\n{}",
            max_len,
            plain
        );
        // Confirm wrap actually occurred: the long item must span >=2 visible
        // lines so the test would fail without the active width override.
        let content_lines = plain.lines().filter(|l| !l.trim().is_empty()).count();
        assert!(
            content_lines >= 3,
            "expected the long item to wrap (>=3 non-empty lines incl. second item), got {}:\n{}",
            content_lines,
            plain
        );
    }

    #[test]
    fn render_image_center_aligned() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::Images, align_policy(renderable::layout::Alignment::Center));
        let md: Markdown = "![alt text|20](nonexistent.png)\n".into();

        let out = page.render(&md).unwrap();
        let plain = crate::testing::strip_ansi_codes(&out);
        let image_line = plain.lines().find(|l| l.contains("IMAGE")).unwrap_or("");
        // The tree fold keeps the raw alt (`alt text|20`) rather than parsing
        // the legacy `|20` width directive, so the placeholder is
        // `▉ IMAGE[alt text|20]`; centered in 80 this leaves ~30 leading spaces.
        let leading = image_line.chars().take_while(|c| *c == ' ').count();
        assert!(
            leading >= 28,
            "image placeholder should be centered (>=28 leading spaces), got {leading}: {:?}",
            image_line
        );
    }

    #[test]
    fn zero_config_with_non_default_alignment_still_matches() {
        // When only alignment is set (no margin/padding/bg/max-width), the page
        // should still render successfully and alignment should be applied.
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::CodeBlocks, align_policy(renderable::layout::Alignment::Center));
        let md: Markdown = "```rust\nfn main() {}\n```\n".into();

        let out = page.render(&md).unwrap();
        assert!(!out.is_empty());
    }

    // ---------- Phase 4: browser rendering tests ----------

    #[test]
    fn zero_config_browser_render_no_wrapper() {
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term);
        let md: Markdown = "# Hello World\n\nSome prose here.\n".into();

        let page_html = page.render_to_browser(&md).unwrap();

        // Zero-config page should not add a wrapper div.
        assert!(
            !page_html.contains("<div class=\"darkmatter-page\""),
            "zero-config page should not add wrapper"
        );
        // But should still contain the rendered markdown. The render-tree
        // browser path emits a heading slug `id`, so match the heading by its
        // tag + text rather than pinning the legacy attribute-free `<h1>`.
        assert!(
            page_html.contains("<h1 id=\"hello-world\">Hello World</h1>"),
            "zero-config page should still render markdown; html={page_html}"
        );
    }

    #[test]
    fn browser_render_with_margin_padding_bg_wraps() {
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term)
            .with_margin(2)
            .with_padding(1)
            .with_page_background(PageBackground::Subtle);
        let md: Markdown = "# Hello\n".into();

        let html = page.render_to_browser(&md).unwrap();
        // Should contain the wrapper div.
        assert!(html.contains("<div class=\"darkmatter-page\""));
        // Should have margin style.
        assert!(html.contains("margin: 2ch 2ch 2ch 2ch"));
        // Should have padding style.
        assert!(html.contains("padding: 1ch 1ch 1ch 1ch"));
        // Should have background color for subtle dark (default is dark mode).
        assert!(html.contains("background-color: rgb(30,30,35)"));
        // Should close the wrapper.
        assert!(html.contains("</div>"));
    }

    #[test]
    fn browser_render_with_max_width() {
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term).with_max_width(100);
        let md: Markdown = "# Hello\n".into();

        let html = page.render_to_browser(&md).unwrap();
        assert!(html.contains("<div class=\"darkmatter-page\""));
        assert!(html.contains("max-width: 100ch"));
        // Default (zero) side margins center the capped frame via `auto` sides.
        assert!(
            html.contains("margin: 0ch auto 0ch auto"),
            "max-width frame with default margins should center via auto sides: {html}"
        );
    }

    #[test]
    fn browser_render_authored_side_margins_suppress_centering() {
        let term = Terminal::new_optimistic(120);
        // Explicit side margins are the author's horizontal placement; the frame
        // keeps them verbatim instead of overriding with `auto` centering.
        let page = DarkmatterPage::new(&term).with_margin_x(3).with_max_width(100);
        let md: Markdown = "# Hello\n".into();

        let html = page.render_to_browser(&md).unwrap();
        assert!(html.contains("margin: 0ch 3ch 0ch 3ch"), "authored side margins must be preserved: {html}");
        assert!(!html.contains("auto"), "authored side margins must suppress auto-centering: {html}");
    }

    #[test]
    fn browser_render_with_pronounced_bg() {
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term).with_page_background(PageBackground::Pronounced);
        let md: Markdown = "# Hello\n".into();

        let html = page.render_to_browser(&md).unwrap();
        // Default color mode is Dark, pronounced on dark => near-white.
        assert!(html.contains("background-color: rgb(245,245,245)"));
    }

    /// Component policies whose target components are absent from the document
    /// (here: no table, no block quote) must not add a page wrapper. The wrapper
    /// is page-frame chrome; an unmatched policy adds no per-component CSS and so
    /// no wrapper (review-1 finding 2).
    #[test]
    fn browser_render_with_unmatched_alignment_policy_adds_no_wrapper() {
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::Tables, align_policy(renderable::layout::Alignment::Center))
            .with_component_policy(PageComponent::BlockQuotes, align_policy(renderable::layout::Alignment::Right));
        let md: Markdown = "# Hello\n".into();

        let html = page.render_to_browser(&md).unwrap();
        assert!(
            !html.contains("<div class=\"darkmatter-page\""),
            "an unmatched component policy must not add a page wrapper; html={html}"
        );
        assert!(html.contains("Hello"));
    }

    #[test]
    fn browser_render_with_unmatched_fill_policy_adds_no_wrapper() {
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::CodeBlocks, max_width_policy(60))
            .with_component_policy(PageComponent::Images, pad_policy(4));
        let md: Markdown = "# Hello\n".into();

        let html = page.render_to_browser(&md).unwrap();
        assert!(
            !html.contains("<div class=\"darkmatter-page\""),
            "an unmatched component policy must not add a page wrapper; html={html}"
        );
        assert!(html.contains("Hello"));
    }

    #[test]
    fn render_to_browser_emits_markdown_html() {
        let term = Terminal::new_optimistic(80);
        let md: Markdown = "# Browser\n".into();
        let page = DarkmatterPage::new(&term);

        // Browser output goes through the inherent method, not a
        // `BrowserRenderable` impl (decisions.md item 12A).
        let html = page.render_to_browser(&md).unwrap();
        // The render-tree browser path emits a heading slug `id`.
        assert!(
            html.contains("<h1 id=\"browser\">Browser</h1>"),
            "render_to_browser should output markdown HTML content; html={html}"
        );
        // Zero-config page should not add a wrapper div.
        assert!(
            !html.contains("<div class=\"darkmatter-page\""),
            "a zero-config page should not add a wrapper"
        );
    }

    #[test]
    fn browser_render_error_for_max_width_zero() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term).with_max_width(0);
        let md: Markdown = "# Hello\n".into();

        let err = page.render_to_browser(&md).unwrap_err();
        assert_eq!(err, PageRenderError::MaxWidthZero);
    }

    #[test]
    fn browser_render_error_for_margins_exceed_width() {
        let term = Terminal::new_optimistic(10);
        let page = DarkmatterPage::new(&term)
            .with_margin_x(5)
            .with_padding_x(1);
        let md: Markdown = "# Hello\n".into();

        let err = page.render_to_browser(&md).unwrap_err();
        assert!(matches!(
            err,
            PageRenderError::MarginsExceedTerminalWidth { .. }
        ));
    }

    // ---------- Phase 6: error reachability tests ----------

    /// A malformed code-block directive (an invalid highlight range) is a fatal
    /// error on the browser render, matching the legacy `output::as_html`
    /// contract (`parse_code_info(...)?`). The `as_html` cutover restores this
    /// via the `validate_code_directives` preflight, which runs over the folded
    /// tree before rendering and surfaces `MarkdownError::InvalidLineRange`; the
    /// `MarkdownError -> PageRenderError::Render` mapping in `render_to_browser`
    /// then propagates it. (The render-tree *terminal* path still degrades, also
    /// matching legacy, which used `unwrap_or_default` there.)
    #[test]
    fn render_browser_errors_on_malformed_code_directive() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        let md: Markdown = "```rust highlight=1-2-3\nfn main() {}\n```\n".into();

        let err = page
            .render_to_browser(&md)
            .expect_err("malformed directive must fail the browser render");
        assert!(
            matches!(err, PageRenderError::Render(_)),
            "malformed directive must map to PageRenderError::Render; got {err:?}"
        );
    }

    /// A malformed disclosure block raises `MarkdownError::MalformedDisclosure`
    /// during the fold (`run_sub_fold` propagates the block-extension error), so
    /// unlike a malformed code directive — which only the browser preflight
    /// catches — it fails the terminal `render` path too. The
    /// `MarkdownError -> PageRenderError::Render` mapping must carry the
    /// malformed-disclosure reason through page rendering (review-5 finding #4).
    #[test]
    fn render_errors_on_malformed_disclosure() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        // Empty summary region between `::disclosure` and `::details`.
        let md: Markdown = "::disclosure\n::details\nbody\n::end-disclosure\n".into();

        let err = page
            .render(&md)
            .expect_err("malformed disclosure must fail the page render");
        let PageRenderError::Render(msg) = err else {
            panic!("malformed disclosure must map to PageRenderError::Render; got {err:?}");
        };
        assert!(
            msg.contains("Malformed disclosure"),
            "error must carry the malformed-disclosure reason; got {msg:?}"
        );
    }

    // ---------- Phase 6: pronounced background test ----------

    #[test]
    fn pronounced_background_on_dark_terminal_inverts_color_mode() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term).with_page_background(PageBackground::Pronounced);
        let md: Markdown = "# Hello\n".into();

        let out = page.render(&md).unwrap();
        // Pronounced on dark terminal => near-white background (245,245,245)
        assert!(
            out.contains("\x1b[48;2;245;245;245m"),
            "pronounced background on dark terminal should emit near-white bg"
        );
    }

    // ---------- Phase 6: regression tests for zero-config equivalence ----------

    // ---------- Phase 6: end-to-end snapshot test ----------

    #[test]
    fn end_to_end_example_from_spec() {
        // Terminal is dark mode, 120 cols wide.
        // md doc.md --margin 2 --padding 1 --page-bg subtle --max-width 100 --line-numbers true --align-code-blocks center
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term)
            .with_margin(2)
            .with_padding(1)
            .with_page_background(PageBackground::Subtle)
            .with_max_width(100)
            .use_line_numbers()
            .with_component_policy(PageComponent::CodeBlocks, align_policy(renderable::layout::Alignment::Center));
        let md: Markdown = "# Title\n\nSome prose here.\n\n```rust\nfn main() {}\n```\n".into();

        let out = page.render(&md).unwrap();
        let lines: Vec<&str> = out.lines().collect();

        // 2 transparent rows top (margin)
        assert!(
            lines[0].trim().is_empty(),
            "first line should be top margin"
        );
        assert!(
            lines[1].trim().is_empty(),
            "second line should be top margin"
        );

        // 1 subtle-bg row (top padding)
        assert!(
            lines[2].contains("\x1b[48;2;30;30;35m"),
            "third line should be top padding with subtle bg"
        );

        // Content should start after margin + padding
        let content_start = 3;
        assert!(
            lines[content_start].contains("Title"),
            "content should start after margin and padding"
        );

        // Find the last content line and verify padding/margin after it
        let _last_content_idx = lines.len() - 4; // 1 bottom padding + 2 bottom margin + trailing newline handling

        // Bottom padding row
        let bottom_padding_idx = lines.len() - 3;
        assert!(
            lines[bottom_padding_idx].contains("\x1b[48;2;30;30;35m"),
            "line before bottom margin should be bottom padding with subtle bg"
        );

        // Bottom margin rows
        assert!(
            lines[lines.len() - 2].trim().is_empty(),
            "second-to-last line should be bottom margin"
        );
        assert!(
            lines[lines.len() - 1].trim().is_empty(),
            "last line should be bottom margin"
        );

        // Verify effective width is capped at 100
        // Each content row should have: 2 margin + 1 padding + content + 1 padding + 2 margin + surplus
        let content_line = lines[content_start];
        let plain = crate::testing::strip_ansi_codes(content_line);
        assert!(
            plain.len() <= 120,
            "content line should not exceed terminal width"
        );
    }

    // ---------- Phase 4: list split + wiring tests ----------

    #[test]
    fn render_ul_left_margin() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::Ul, left_margin_policy(4));
        let md: Markdown = "- Hello world\n".into();

        let out = page.render(&md).unwrap();
        let plain = crate::testing::strip_ansi_codes(&out);
        let list_line = plain.lines().find(|l| l.contains("Hello")).unwrap();
        assert!(
            list_line.starts_with("    - "),
            "unordered list should have 4ch left margin before marker, got: {:?}",
            list_line
        );
    }

    #[test]
    fn render_ul_max_width() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::Ul, max_width_policy(40));
        let md: Markdown = "- This is an unusually long bullet item that ought to be forced to wrap once Max(40) constrains the list rendering width to forty columns.\n".into();

        let out = page.render(&md).unwrap();
        let plain = crate::testing::strip_ansi_codes(&out);
        let lines: Vec<&str> = plain.lines().collect();
        let max_len = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        assert!(
            max_len <= 40,
            "list lines should be capped to 40 cols, got max={}:\n{}",
            max_len,
            plain
        );
    }

    #[test]
    fn render_ul_left_margin_and_max_width() {
        let term = Terminal::new_optimistic(80);
        let mut policy = max_width_policy(40);
        policy.layout.margin.left = renderable::layout::TargetValue::universal(renderable::layout::Length::ch(4));
        let page = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::Ul, policy);
        let md: Markdown = "- This is an unusually long bullet item that ought to be forced to wrap once Max(40) constrains the list rendering width to forty columns.\n".into();

        let out = page.render(&md).unwrap();
        let plain = crate::testing::strip_ansi_codes(&out);
        let lines: Vec<&str> = plain.lines().collect();
        // Body wraps at <= ul.max-width (40 cells); the 4-cell left margin
        // sits outside the body, so total line length is <= 44 cells.
        let max_total = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        assert!(
            max_total <= 44,
            "list lines should fit in left-margin (4) + body (<= 40) = 44 cols, got max={}:\n{}",
            max_total,
            plain
        );
        // Body width: stripping the 4-cell margin, the remaining content
        // must wrap at no more than 40 cells.
        let max_body = lines
            .iter()
            .filter(|l| !l.trim().is_empty())
            .map(|l| {
                let trimmed = l.strip_prefix("    ").unwrap_or(l);
                trimmed.chars().count()
            })
            .max()
            .unwrap_or(0);
        assert!(
            max_body <= 40,
            "body (after 4ch margin) should wrap at <= 40 cols, got max body={}:\n{}",
            max_body,
            plain
        );
        // First non-empty line should start with 4 spaces of left margin.
        let first_line = lines.iter().find(|l| !l.trim().is_empty()).copied().unwrap_or("");
        assert!(
            first_line.starts_with("    - "),
            "first line should start with 4ch left margin, got: {:?}",
            first_line
        );
    }

    #[test]
    fn render_ol_alignment_right() {
        let term = Terminal::new_optimistic(80);
        let mut policy = max_width_policy(40);
        policy.layout.alignment = renderable::layout::Alignment::Right;
        let page = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::Ol, policy);
        let md: Markdown = "1. Hello world\n".into();

        let out = page.render(&md).unwrap();
        let plain = crate::testing::strip_ansi_codes(&out);
        let list_line = plain.lines().find(|l| l.contains("Hello")).unwrap();
        // Component is 40 cols wide, right-aligned in 80 => 40 cols of left padding.
        let leading_spaces = list_line.len() - list_line.trim_start().len();
        assert!(
            leading_spaces >= 35,
            "ordered list should be right-aligned, got {} leading spaces: {:?}",
            leading_spaces,
            list_line
        );
    }

    #[test]
    fn render_li_body_alignment_right() {
        let term = Terminal::new_optimistic(80);
        let mut policy = max_width_policy(40);
        policy.layout.alignment = renderable::layout::Alignment::Right;
        let page = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::Li, policy);
        let md: Markdown = "- Hello world\n".into();

        assert!(!page.is_default_layout(), "page should not be default layout");
        assert_eq!(
            page.component_policy(PageComponent::Li).and_then(|p| p.layout.max_width.as_ref()).map(edge_ch),
            Some(40)
        );
        assert_eq!(
            page.component_policy(PageComponent::Li).map(|p| p.layout.alignment).unwrap_or_default(),
            renderable::layout::Alignment::Right
        );

        let out = page.render(&md).unwrap();
        let plain = crate::testing::strip_ansi_codes(&out);
        let lines: Vec<&str> = plain.lines().collect();
        // Per spec, `li.alignment` affects the item body only; the marker
        // stays at the column dictated by the containing Ul (column 0 here,
        // since Ul has no override). The body becomes a block on a new line
        // that is right-aligned within `effective_width - body_width = 40`.
        let marker_line = lines
            .iter()
            .find(|l| l.trim_start().starts_with('-'))
            .copied()
            .unwrap_or("");
        assert!(
            marker_line.starts_with("- "),
            "marker should remain at column 0 (Ul column), got: {:?}",
            marker_line
        );
        let body_line = lines
            .iter()
            .find(|l| l.contains("Hello"))
            .copied()
            .unwrap_or("");
        assert!(
            !body_line.contains('-'),
            "body should not contain the marker (marker is on its own line): {:?}",
            body_line
        );
        let leading_spaces = body_line.len() - body_line.trim_start().len();
        assert!(
            leading_spaces >= 35,
            "li body should be right-aligned within effective_width, got {} leading spaces: {:?}",
            leading_spaces,
            body_line
        );
    }

    #[test]
    fn browser_selectors_split_for_lists() {
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::Ul, align_policy(renderable::layout::Alignment::Center))
            .with_component_policy(PageComponent::Ol, align_policy(renderable::layout::Alignment::Right))
            .with_component_policy(PageComponent::Li, max_width_policy(30));
        let md: Markdown = "- item\n".into();

        let html = page.render_to_browser(&md).unwrap();
        // The matched `li` max-width lowers to inline CSS on the `<li>`; no page
        // wrapper is added for a component-policy-only page (review-1 finding 2).
        assert!(
            !html.contains("<div class=\"darkmatter-page\""),
            "component policy alone must not add a page wrapper; html={html}"
        );
        assert!(html.contains("max-width:30ch"), "li max-width must be inline; html={html}");
        assert!(html.contains("item"));
    }

    #[test]
    fn browser_ul_left_margin_css() {
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::Ul, left_margin_policy(4));
        let md: Markdown = "- item\n".into();

        let html = page.render_to_browser(&md).unwrap();
        // The `ul` left margin lowers to inline CSS on the `<ul>`; no wrapper.
        assert!(
            !html.contains("<div class=\"darkmatter-page\""),
            "component policy alone must not add a page wrapper; html={html}"
        );
        assert!(html.contains("margin-left:4ch"), "ul left margin must be inline; html={html}");
        assert!(html.contains("item"));
    }

    #[test]
    fn li_independent_of_ul_ol() {
        let term = Terminal::new_optimistic(80);
        let mut ul_policy = max_width_policy(30);
        ul_policy.layout.alignment = renderable::layout::Alignment::Left;
        let mut ol_policy = max_width_policy(40);
        ol_policy.layout.alignment = renderable::layout::Alignment::Center;
        let mut li_policy = max_width_policy(50);
        li_policy.layout.alignment = renderable::layout::Alignment::Right;
        let page = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::Ul, ul_policy)
            .with_component_policy(PageComponent::Ol, ol_policy)
            .with_component_policy(PageComponent::Li, li_policy);

        // Each component retains its own alignment independently.
        assert_eq!(page.component_policy(PageComponent::Ul).map(|p| p.layout.alignment).unwrap_or_default(), renderable::layout::Alignment::Left);
        assert_eq!(page.component_policy(PageComponent::Ol).map(|p| p.layout.alignment).unwrap_or_default(), renderable::layout::Alignment::Center);
        assert_eq!(page.component_policy(PageComponent::Li).map(|p| p.layout.alignment).unwrap_or_default(), renderable::layout::Alignment::Right);

        // Each component retains its own fill independently.
        assert_eq!(
            page.component_policy(PageComponent::Ul).and_then(|p| p.layout.max_width.as_ref()).map(edge_ch),
            Some(30)
        );
        assert_eq!(
            page.component_policy(PageComponent::Ol).and_then(|p| p.layout.max_width.as_ref()).map(edge_ch),
            Some(40)
        );
        assert_eq!(
            page.component_policy(PageComponent::Li).and_then(|p| p.layout.max_width.as_ref()).map(edge_ch),
            Some(50)
        );
    }

    // ---------- Phase 1: color API tests ----------

    use renderable::color::{Color, Tailwind};
    use renderable::style::PaintColor;

    fn red_color() -> PaintColor {
        PaintColor::new(Color::Tailwind(Tailwind::Red500))
    }

    fn blue_color() -> PaintColor {
        PaintColor::new(Color::Tailwind(Tailwind::Blue500))
    }

    #[test]
    fn color_setters_and_getters() {
        let page = page()
            .with_page_color(red_color())
            .with_page_bg_color(blue_color())
            .with_component_color(PageComponent::Tables, red_color())
            .with_component_bg_color(PageComponent::Tables, blue_color());

        assert_eq!(page.page_color(), Some(&red_color()));
        assert_eq!(page.page_bg_color(), Some(&blue_color()));
        assert_eq!(
            page.color_for(PageComponent::Tables),
            Some(&red_color())
        );
        assert_eq!(
            page.bg_color_for(PageComponent::Tables),
            Some(&blue_color())
        );
    }

    #[test]
    fn color_inheritance_from_page() {
        let page = page()
            .with_page_color(red_color())
            .with_page_bg_color(blue_color());

        // Components without explicit color inherit page color.
        assert_eq!(
            page.color_for(PageComponent::Tables),
            Some(&red_color())
        );
        assert_eq!(
            page.bg_color_for(PageComponent::Tables),
            Some(&blue_color())
        );
        assert_eq!(
            page.color_for(PageComponent::Hyperlinks),
            Some(&red_color())
        );
    }

    #[test]
    fn component_color_overrides_page_color() {
        let page = page()
            .with_page_color(red_color())
            .with_component_color(PageComponent::Tables, blue_color());

        assert_eq!(
            page.color_for(PageComponent::Tables),
            Some(&blue_color())
        );
        // Other components still inherit page color.
        assert_eq!(
            page.color_for(PageComponent::Images),
            Some(&red_color())
        );
    }

    #[test]
    fn component_bg_color_overrides_page_bg_color() {
        let page = page()
            .with_page_bg_color(red_color())
            .with_component_bg_color(PageComponent::Tables, blue_color());

        assert_eq!(
            page.bg_color_for(PageComponent::Tables),
            Some(&blue_color())
        );
        assert_eq!(
            page.bg_color_for(PageComponent::Images),
            Some(&red_color())
        );
    }

    #[test]
    fn color_only_page_is_not_default_layout() {
        let page = page().with_page_color(red_color());
        assert!(!page.is_default_layout(), "page with color should not be default");
    }

    #[test]
    fn bg_color_only_page_is_not_default_layout() {
        let page = page().with_page_bg_color(red_color());
        assert!(!page.is_default_layout(), "page with bg-color should not be default");
    }

    #[test]
    fn component_color_only_page_is_not_default_layout() {
        let page = page().with_component_color(PageComponent::Tables, red_color());
        assert!(!page.is_default_layout(), "page with component color should not be default");
    }

    // ---------- Phase 5: render-level color tests ----------

    #[test]
    fn terminal_page_color_applies_sgr_to_components() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term).with_page_color(red_color());
        let md: Markdown = "# Hello\n".into();

        let out = page.render(&md).unwrap();
        // The heading text should be wrapped with the page color SGR
        // and properly reset.
        assert!(
            out.contains("\x1b[38;2;"),
            "page color should emit foreground SGR; got: {out:?}"
        );
        assert!(
            out.contains("\x1b[0m"),
            "page color scope should end with reset; got: {out:?}"
        );
    }

    #[test]
    fn terminal_component_color_overrides_page_color_in_output() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .with_page_color(red_color())
            .with_component_color(PageComponent::Tables, blue_color());
        let md: Markdown = "| a | b |\n|---|---|\n| 1 | 2 |\n".into();

        let out = page.render(&md).unwrap();
        // Table output should contain blue SGR, not just red.
        // Both colors may appear (red for heading, blue for table), so we
        // just verify the table-specific color is present.
        assert!(
            out.contains("\x1b[38;2;"),
            "component color should emit SGR; got: {out:?}"
        );
    }

    #[test]
    fn terminal_color_depth_none_omits_sgr_for_colors() {
        let term = Terminal::new_optimistic(80);
        let md: Markdown = "# Hello\n".into();
        let out = DarkmatterPage::new(&term)
            .with_page_color(red_color())
            .with_color_depth(ColorDepth::None)
            .render(&md)
            .unwrap();

        assert!(
            !out.contains("\x1b[38;2;"),
            "ColorDepth::None must suppress color SGR; got: {out:?}"
        );
    }

    #[test]
    fn terminal_reset_boundary_scopes_component_colors() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .with_component_color(PageComponent::Tables, red_color());
        let md: Markdown = "| a | b |\n|---|---|\n| 1 | 2 |\n".into();

        let out = page.render(&md).unwrap();
        // The table output should be wrapped with an opening SGR and a reset.
        assert!(
            out.contains("\x1b[0m"),
            "component color must be scoped with reset; got: {out:?}"
        );
    }

    #[test]
    fn browser_page_color_emits_inheriting_root_div() {
        // Review-1 finding 3: the page foreground rides the render tree's root
        // node, which the browser fold renders as a wrapping `<div>` so the color
        // inherits to descendants via CSS — it is NOT emitted on the
        // `.darkmatter-page` frame (where it would be a duplicate declaration).
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term).with_page_color(red_color());
        let md: Markdown = "# Hello\n".into();

        let html = page.render_to_browser(&md).unwrap();
        assert!(
            html.contains("<div style=\"color:rgb("),
            "page color should ride a wrapping root <div>; got: {html}"
        );
        // The page frame itself must not carry the foreground color.
        let frame = html
            .split_once("class=\"darkmatter-page\" style=\"")
            .and_then(|(_, rest)| rest.split_once('"'))
            .map(|(style, _)| style)
            .unwrap_or("");
        assert!(
            !frame.contains("color:"),
            "page foreground must not be duplicated on the page frame; frame style: {frame}"
        );
    }

    #[test]
    fn browser_page_bg_color_overrides_page_background_css() {
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term)
            .with_page_background(PageBackground::Subtle)
            .with_page_bg_color(red_color());
        let md: Markdown = "# Hello\n".into();

        let html = page.render_to_browser(&md).unwrap();
        // The wrapper should have the explicit bg-color after the computed one.
        let bg_count = html.matches("background-color:").count();
        assert!(
            bg_count >= 1,
            "wrapper should have background-color; got: {html}"
        );
        assert!(
            html.contains("background-color: rgb("),
            "page bg-color should be rgb(...); got: {html}"
        );
    }

    #[test]
    fn browser_component_color_emits_per_component_css() {
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term)
            .with_component_color(PageComponent::Tables, red_color())
            .with_component_bg_color(PageComponent::BlockQuotes, blue_color());
        let md: Markdown = "# Hello\n\n> Quote\n\n| a | b |\n|---|---|\n| 1 | 2 |\n".into();

        let html = page.render_to_browser(&md).unwrap();
        // Matched component policies lower to inline CSS on the component
        // elements (the fold), not to a page wrapper or a stylesheet rule. The
        // page has no frame geometry, so no wrapper is added (review-1 finding 2).
        assert!(
            !html.contains("<div class=\"darkmatter-page\""),
            "component policy alone must not add a page wrapper; html={html}"
        );
        assert!(
            html.contains("<table style=\"color:rgb(251, 44, 54)\""),
            "table component color must appear as inline CSS; html={html}"
        );
        assert!(
            html.contains("background-color:rgb(43, 127, 255)"),
            "block-quote component bg-color must appear as inline CSS; html={html}"
        );
        assert!(html.contains("Hello"));
        assert!(html.contains("Quote"));
    }

    #[test]
    fn browser_opacity_preserved_as_rgba() {
        let term = Terminal::new_optimistic(120);
        let semi = crate::style::StyleColor {
            color: Color::Tailwind(Tailwind::Red500),
            opacity: Some(50),
        }
        .to_paint_color();
        let page = DarkmatterPage::new(&term).with_page_color(semi);
        let md: Markdown = "# Hello\n".into();

        let html = page.render_to_browser(&md).unwrap();
        assert!(
            html.contains("rgba(") && html.contains("0.5"),
            "opacity should produce rgba CSS; got: {html}"
        );
    }

    #[test]
    fn browser_component_opacity_preserved_as_rgba() {
        // Review-1 finding 1: a component `bg-color` with Tailwind opacity must
        // survive the cutover path to the browser as `rgba(...)` — the renderable
        // `Style` cannot carry opacity, so the browser entry point splices it in.
        let term = Terminal::new_optimistic(120);
        let semi = crate::style::StyleColor {
            color: Color::Tailwind(Tailwind::Red500),
            opacity: Some(50),
        }
        .to_paint_color();
        let page = DarkmatterPage::new(&term)
            .with_component_bg_color(PageComponent::BlockQuotes, semi);
        let md: Markdown = "> Quote\n".into();

        let html = page.render_to_browser(&md).unwrap();
        assert!(
            html.contains("rgba(") && html.contains("0.5"),
            "component bg-color opacity must lower to rgba on the browser path; got: {html}"
        );
    }

    #[test]
    fn terminal_component_opacity_dropped_but_color_kept() {
        // The terminal drops opacity (documented) yet still paints the color.
        let term = Terminal::new_optimistic(80);
        let semi = crate::style::StyleColor {
            color: Color::Tailwind(Tailwind::Red500),
            opacity: Some(50),
        }
        .to_paint_color();
        let page = DarkmatterPage::new(&term)
            .with_component_bg_color(PageComponent::BlockQuotes, semi);
        let md: Markdown = "> Quote\n".into();

        let out = page.render(&md).unwrap();
        assert!(
            out.contains("\x1b[48;2;"),
            "terminal should emit a 24-bit background SGR (opacity dropped); got: {out:?}"
        );
    }

    #[test]
    fn terminal_opacity_dropped_from_sgr() {
        let term = Terminal::new_optimistic(80);
        let semi = crate::style::StyleColor {
            color: Color::Tailwind(Tailwind::Red500),
            opacity: Some(50),
        }
        .to_paint_color();
        let page = DarkmatterPage::new(&term).with_page_color(semi);
        let md: Markdown = "# Hello\n".into();

        let out = page.render(&md).unwrap();
        // SGR should NOT contain opacity; it should be a plain 24-bit color.
        assert!(
            out.contains("\x1b[38;2;"),
            "terminal should still emit 24-bit SGR without opacity; got: {out:?}"
        );
    }

    #[test]
    fn browser_css_special_colors_passthrough() {
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term)
            .with_component_color(PageComponent::Tables, PaintColor::new(
                Color::Tailwind(Tailwind::Transparent),
            ))
            .with_component_color(PageComponent::BlockQuotes, PaintColor::new(
                Color::Tailwind(Tailwind::Current),
            ))
            .with_component_bg_color(PageComponent::Images, PaintColor::new(
                Color::Tailwind(Tailwind::Inherit),
            ));
        let md: Markdown = "# Hello\n\n> Quote\n\n| a | b |\n|---|---|\n| 1 | 2 |\n".into();

        let html = page.render_to_browser(&md).unwrap();
        // The CSS-special keywords pass straight through to inline CSS on the
        // component elements; no page wrapper (review-1 finding 2).
        assert!(
            !html.contains("<div class=\"darkmatter-page\""),
            "component policy alone must not add a page wrapper; html={html}"
        );
        assert!(
            html.contains("color:transparent") && html.contains("color:currentColor"),
            "special color keywords must reach inline CSS; html={html}"
        );
        assert!(html.contains("Hello"));
        assert!(html.contains("Quote"));
    }

    #[test]
    fn browser_list_selectors_emit_separate_rules_with_colors() {
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term)
            .with_component_color(PageComponent::Ul, red_color())
            .with_component_color(PageComponent::Ol, blue_color())
            .with_component_color(PageComponent::Li, PaintColor::new(
                Color::Tailwind(Tailwind::Green500),
            ));
        let md: Markdown = "- one\n\n1. two\n".into();

        let html = page.render_to_browser(&md).unwrap();
        // Per-list colors lower to inline CSS on the `<ul>`/`<ol>`/`<li>`; no
        // page wrapper for a component-policy-only page (review-1 finding 2).
        assert!(
            !html.contains("<div class=\"darkmatter-page\""),
            "component policy alone must not add a page wrapper; html={html}"
        );
        assert!(
            html.contains("<ul style=\"color:rgb(251, 44, 54)\"")
                && html.contains("<ol style=\"color:rgb(43, 127, 255)\"")
                && html.contains("color:rgb(0, 201, 80)"),
            "ul/ol/li component colors must be inline; html={html}"
        );
        assert!(html.contains("one"));
        assert!(html.contains("two"));
    }

    /// A hyperlink *color* must not strip the link's OSC8 clickability when the
    /// render context supports OSC8. Under the review-5 capability model OSC8
    /// availability follows a *deliberate* frame (geometry) or an OSC8-capable
    /// terminal — never the matched color policy itself — so the page carries a
    /// minimal frame (`margin-left`) to select the optimistic profile. The color
    /// is then layered on and must coexist with OSC8, not clobber it.
    #[test]
    fn terminal_hyperlink_color_preserves_osc8_sequences() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .with_margin_left(1)
            .with_hyperlink_mode(HyperlinkMode::Always)
            .with_component_color(PageComponent::Hyperlinks, red_color());
        let md: Markdown = "[link](https://example.com)\n".into();

        let out = page.render(&md).unwrap();
        // OSC8 sequences must still be present.
        assert!(
            out.contains("\x1b]8;;https://example.com\x1b\\")
                || out.contains("\x1b]8;;https://example.com\x07"),
            "OSC8 open sequence must be preserved; got: {out:?}"
        );
        assert!(
            out.contains("\x1b]8;;\x1b\\") || out.contains("\x1b]8;;\x07"),
            "OSC8 close sequence must be preserved; got: {out:?}"
        );
        // The hyperlink text should also have the color SGR applied.
        assert!(
            out.contains("\x1b[38;2;"),
            "hyperlink color SGR should be present; got: {out:?}"
        );
    }

    #[test]
    fn code_block_bg_color_override_does_not_clobber_highlighting() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .with_component_bg_color(PageComponent::CodeBlocks, red_color());
        let md: Markdown = "```rust\nfn main() {}\n```\n".into();

        let out = page.render(&md).unwrap();
        // The code block should still contain syntax-highlighting SGRs
        // (multiple different colors for keywords, identifiers, etc.).
        let sgr_count = out.matches("\x1b[38;2;").count();
        assert!(
            sgr_count >= 2,
            "code block should retain multiple syntax highlight colors; got: {out:?}"
        );
    }

    // ---------- Review-5 follow-ups: terminal layout fidelity ----------

    /// With `ColorDepth::None`, a styled page must still render the full
    /// table layout (box-drawing characters and cell contents) — the
    /// pipeline no longer falls back to raw Markdown source.
    #[test]
    fn color_depth_none_preserves_table_layout_when_page_color_set() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .with_page_color(red_color())
            .with_color_depth(ColorDepth::None);
        let md: Markdown = "| H |\n|---|\n| C |\n".into();

        let out = page.render(&md).unwrap();
        assert!(
            !out.contains("\x1b[38;2;"),
            "ColorDepth::None must suppress color SGR even with style.page.color; got: {out:?}"
        );
        assert!(
            out.contains('H') && out.contains('C'),
            "table cell text must survive ColorDepth::None; got: {out:?}"
        );
        assert!(
            out.contains('┌') || out.contains('+') || out.contains('|'),
            "table structure must render under ColorDepth::None; got: {out:?}"
        );
    }

    /// `style.ul.color` must apply to list-item body text even when
    /// `style.li.color` is unset — list items inherit through their
    /// container scope just like CSS. The Tailwind Red-500 SGR triplet
    /// resolves at render time, so we look up the canonical bytes from the
    /// shared lowering helper rather than hard-coding RGB values.
    #[test]
    fn ul_color_inherits_into_li_body_when_li_color_unset() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .with_component_color(PageComponent::Ul, red_color());
        let md: Markdown = "- alpha\n- beta\n".into();

        let out = page.render(&md).unwrap();
        let red_sc = crate::style::StyleColor {
            color: red_color().color,
            opacity: None,
        };
        let red_sgr = crate::style::lower_to_sgr(&red_sc, ColorDepth::TrueColor, false)
            .expect("red_color must lower to truecolor SGR");
        // The ul color should wrap the marker AND the body, even though the
        // li scope has no explicit color of its own (the body would
        // otherwise inherit a None scope from li).
        let occurrences = out.matches(&red_sgr).count();
        assert!(
            occurrences >= 2,
            "ul color must wrap each item's body; got: {out:?}"
        );
    }

    /// `style.hyperlinks.color` must wrap link label text inside table
    /// cells, while preserving the OSC8 sequence — and it overrides the
    /// surrounding table color.
    #[test]
    fn browser_hr_bg_color_targets_rendered_element() {
        // The HR component emits `<svg class="darkmatter-hr">`. Under the
        // fold-based behavior the per-component CSS rule is no longer emitted,
        // but the SVG class remains so downstream stylesheets can target it.
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term)
            .with_component_bg_color(PageComponent::Hr, red_color());
        let md: Markdown = "Before\n\n---\n\nAfter\n".into();

        let html = page.render_to_browser(&md).unwrap();

        // Fold-based behavior: no bespoke per-component CSS, but the SVG
        // still carries the class for external stylesheets.
        assert!(
            html.contains(r#"class="darkmatter-hr""#),
            "HR SVG must carry the `darkmatter-hr` class; got: {html}"
        );
    }

    #[test]
    fn browser_hr_color_emits_rule_for_svg_target() {
        // `style.hr.color` is baked onto the thematic-break node and lowered
        // into the SVG's `--hr-color` custom property (and the primitives'
        // `var(--hr-color, …)` fallback), so the rule paints red without any
        // page wrapper or per-component stylesheet rule (review-1 finding 2).
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term)
            .with_component_color(PageComponent::Hr, red_color());
        let md: Markdown = "---\n".into();

        let html = page.render_to_browser(&md).unwrap();

        assert!(
            !html.contains("<div class=\"darkmatter-page\""),
            "component policy alone must not add a page wrapper; html={html}"
        );
        assert!(
            html.contains("--hr-color: rgb(251, 44, 54)"),
            "hr component color must reach the SVG `--hr-color`; html={html}"
        );
    }

    /// A hyperlink color applied to a link nested in a colored table cell must
    /// wrap the link text *and* leave its OSC8 clickability intact. As in
    /// [`terminal_hyperlink_color_preserves_osc8_sequences`], OSC8 availability
    /// follows a deliberate frame under the review-5 capability model, so the
    /// page carries a minimal `margin-left` frame; the matched colors are then
    /// local node attrs that coexist with the OSC8 the frame's profile provides.
    #[test]
    fn hyperlink_color_applies_inside_table_cells() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .with_margin_left(1)
            .with_hyperlink_mode(HyperlinkMode::Always)
            .with_component_color(PageComponent::Tables, blue_color())
            .with_component_color(PageComponent::Hyperlinks, red_color());
        let md: Markdown = "| col |\n|---|\n| [click](https://example.com) |\n".into();

        let out = page.render(&md).unwrap();
        // OSC8 sequences must still be present so the link remains clickable.
        assert!(
            out.contains("\x1b]8;;https://example.com\x07")
                || out.contains("\x1b]8;;https://example.com\x1b\\"),
            "OSC8 open sequence must be preserved in table; got: {out:?}"
        );
        let red_sc = crate::style::StyleColor {
            color: red_color().color,
            opacity: None,
        };
        let red_sgr = crate::style::lower_to_sgr(&red_sc, ColorDepth::TrueColor, false)
            .expect("red_color must lower to truecolor SGR");
        assert!(
            out.contains(&red_sgr),
            "hyperlink color must wrap table-link text; got: {out:?}"
        );
    }

    // ---------- Phase 5: renderable-typed page frame ----------

    #[test]
    fn page_frame_stores_renderable_types() {
        let page = DarkmatterPage::new(&Terminal::new_optimistic(80))
            .with_margin(2)
            .with_padding(3);
        // page-frame margin/padding are renderable Edges, not PageMargin/PagePadding
        let _: &renderable::layout::Edges = page.page_margin();
        let _: &renderable::layout::Edges = page.page_padding();
    }

    #[test]
    fn length_to_cells_resolves_percent_against_base() {
        use renderable::layout::{Length, TargetValue};
        assert_eq!(length_to_cells(&TargetValue::universal(Length::Percent(10.0)), 80), 8);
        assert_eq!(length_to_cells(&TargetValue::universal(Length::ch(4)), 80), 4);
        assert_eq!(length_to_cells(&TargetValue::universal(Length::Zero), 80), 0);
    }

    #[test]
    fn length_to_css_frame_emits_authored_unit() {
        use renderable::layout::{Length, TargetValue};
        assert_eq!(length_to_css_frame(&TargetValue::universal(Length::Percent(50.0))), "50%");
        assert_eq!(length_to_css_frame(&TargetValue::universal(Length::ch(12))), "12ch");
        assert_eq!(length_to_css_frame(&TargetValue::universal(Length::Zero)), "0ch");
    }

    #[test]
    fn percent_frame_browser_emits_percent_terminal_resolves_cells() {
        // Review-1 finding 3: the page frame retains the authored `Length`.
        use renderable::layout::Length;
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .with_margin_left_length(Length::Percent(10.0))
            .with_max_width_length(Length::Percent(50.0));
        let md: Markdown = "Hello world\n".into();

        // Browser: authored percentages survive to CSS (resolve against viewport).
        let html = page.render_to_browser(&md).unwrap();
        assert!(html.contains("max-width: 50%"), "browser must emit percent max-width; got: {html}");
        assert!(html.contains("10%"), "browser must emit percent margin; got: {html}");

        // Terminal: percentages resolve to cells. content = 80 - 8 (10% margin) = 72;
        // max-width = 50% of 72 = 36.
        assert_eq!(page.max_width(), Some(36), "terminal must resolve percent max-width to cells");
        page.render(&md).expect("decorated percent frame must render on the terminal");
    }

    #[test]
    fn pronounced_still_flips_render_mode() {
        let page = DarkmatterPage::new(&Terminal::new_optimistic(80))
            .with_page_background(PageBackground::Pronounced);
        // existing guard: the code theme mode inverts; reuse the existing snapshot
        let html = page.render_to_browser(&"```rust\nfn x(){}\n```".into()).unwrap();
        assert!(html.contains("darkmatter-page"));
        insta::assert_snapshot!("pronounced_background_snapshot", html);
    }

    // ---------- Phase 3: ColorMode::Unknown fallback tests ----------

    /// `ColorMode::Unknown` page/prose must fall back to the configured page
    /// mode (default `Dark`); the page surface inverts against the captured
    /// terminal mode when one is present, but a standalone `DarkmatterPage`
    /// built from a `Terminal` whose color mode is `Unknown` renders as dark
    /// (Decision #6 in the spec).
    #[test]
    fn color_mode_unknown_page_prose_defaults_to_dark() {
        let mut term = Terminal::new_optimistic(80);
        term.color_mode = biscuit_terminal::discovery::detection::ColorMode::Unknown;
        let page = DarkmatterPage::new(&term);
        let md: Markdown = "```rust\nfn codemarker() {}\n```\n".into();

        let out = page.with_margin_left(1).render(&md).unwrap();
        let panel_bg = active_bg_at(&out, "codemarker");
        let lum = rel_luminance(panel_bg.0, panel_bg.1, panel_bg.2);
        // A dark page => the default inverse code panel resolves to LIGHT (the
        // unknown mode's `inverted()` resolves to `Light`).
        assert!(
            lum > 0.5,
            "an unknown-color-mode terminal must fall back to a dark page \
             and invert the code panel to light; panel bg {panel_bg:?} (lum {lum:.3})"
        );
    }

    /// An explicit `with_color_mode(Dark)` on a page built from an unknown
    /// terminal must keep the page dark, so the inverse code panel still
    /// resolves light. This pins the `with_color_mode` precedence.
    #[test]
    fn color_mode_unknown_with_explicit_dark_inverts_to_light() {
        let mut term = Terminal::new_optimistic(80);
        term.color_mode = biscuit_terminal::discovery::detection::ColorMode::Unknown;
        let page = DarkmatterPage::new(&term).with_color_mode(ColorMode::Dark);
        let md: Markdown = "```rust\nfn codemarker() {}\n```\n".into();

        let out = page.with_margin_left(1).render(&md).unwrap();
        let panel_bg = active_bg_at(&out, "codemarker");
        let lum = rel_luminance(panel_bg.0, panel_bg.1, panel_bg.2);
        assert!(
            lum > 0.5,
            "explicit ColorMode::Dark with an unknown terminal must keep the \
             page dark and invert the panel to light; panel bg {panel_bg:?} (lum {lum:.3})"
        );
    }

    /// An explicit `with_color_mode(Light)` on a page built from an unknown
    /// terminal must keep the page light, so the inverse code panel resolves
    /// to dark. The unspecified terminal mode falls back to the configured
    /// `with_color_mode` rather than `Dark`.
    #[test]
    fn color_mode_unknown_with_explicit_light_inverts_to_dark() {
        let mut term = Terminal::new_optimistic(80);
        term.color_mode = biscuit_terminal::discovery::detection::ColorMode::Unknown;
        let page = DarkmatterPage::new(&term).with_color_mode(ColorMode::Light);
        let md: Markdown = "```rust\nfn codemarker() {}\n```\n".into();

        let out = page.with_margin_left(1).render(&md).unwrap();
        let panel_bg = active_bg_at(&out, "codemarker");
        let lum = rel_luminance(panel_bg.0, panel_bg.1, panel_bg.2);
        assert!(
            lum < 0.5,
            "explicit ColorMode::Light with an unknown terminal must keep the \
             page light and invert the panel to dark; panel bg {panel_bg:?} (lum {lum:.3})"
        );
    }

    /// The default code-block mode is `Inverse`: a dark page inverts to a
    /// light code panel. The contract must hold under `ColorMode::Unknown`
    /// as well as `Dark` / `Light` (Decision #6).
    #[test]
    fn color_mode_unknown_default_inverse_code_block_resolves_light() {
        let mut term = Terminal::new_optimistic(80);
        term.color_mode = biscuit_terminal::discovery::detection::ColorMode::Unknown;
        // No `with_color_mode` override: the captured unknown mode resolves
        // to `Dark` per the layout context's surface_mode mapping, so the
        // default inverse code block must render in a light theme.
        let page = DarkmatterPage::new(&term);
        let md: Markdown = "```rust\nfn codemarker() {}\n```\n".into();

        let out = page.with_margin_left(1).render(&md).unwrap();
        let panel_bg = active_bg_at(&out, "codemarker");
        let lum = rel_luminance(panel_bg.0, panel_bg.1, panel_bg.2);
        assert!(
            lum > 0.5,
            "ColorMode::Unknown with the default inverse code block must \
             resolve the panel to a light theme; panel bg {panel_bg:?} (lum {lum:.3})"
        );
    }

    // ================================================================
    // Phase 5.1 — cross-surface contrast guardrail
    //
    // Locks the contract that fenced code blocks always separate visually
    // from the page surface, on both terminal and browser targets, in
    // both light and dark modes — and specifically when the real
    // `Terminal::color_mode` and the page's `with_color_mode` option
    // disagree. Decision #4 in the spec: `Terminal` is the source of
    // truth; the option is the fallback for `Unknown`. The pre-fix
    // defect pinned the panel against the option mode and the page
    // surface against the terminal mode, so the two surfaces drifted.
    // ================================================================

    /// Pull the `.code-block` `background-color` rule out of a browser-render
    /// HTML string. Falls back to looking for the rule in any `<style>` block the
    /// render emits, and panics with a useful message if the rule is absent.
    /// The value can be either `rgb(R, G, B)` or `#rrggbb`; we accept both.
    fn browser_code_block_bg(html: &str) -> (u8, u8, u8) {
        let rule = ".code-block{background-color:";
        let idx = html
            .find(rule)
            .unwrap_or_else(|| panic!("missing {rule:?} in render:\n{html}"));
        let rest = &html[idx + rule.len()..];
        let end = rest.find(';').expect("unterminated CSS rule");
        parse_css_color(&rest[..end])
    }

    /// Pull the page-wrapper `background-color` out of the
    /// `<div class="darkmatter-page" style="...">` declaration. A zero (the
    /// default `PageBackground::Transparent`) skips the rule entirely; the test
    /// pins a non-zero background so the wrapper rule is always emitted.
    fn browser_page_wrapper_bg(html: &str) -> (u8, u8, u8) {
        let marker = "<div class=\"darkmatter-page\" style=\"";
        let start = html
            .find(marker)
            .unwrap_or_else(|| panic!("missing {marker:?} in render:\n{html}"))
            + marker.len();
        let rest = &html[start..];
        let end = rest.find('\"').expect("unterminated style attr");
        let attrs = &rest[..end];
        let needle = "background-color:";
        let idx = attrs
            .find(needle)
            .unwrap_or_else(|| panic!("missing page-wrapper background-color in: {attrs}"));
        let tail = &attrs[idx + needle.len()..];
        // Trim leading whitespace, take up to the first `;`.
        let trimmed = tail.trim_start();
        let end = trimmed.find(';').expect("unterminated CSS declaration");
        parse_css_color(&trimmed[..end])
    }

    /// Parse a CSS color value, accepting both `rgb(R, G, B)` and `#rrggbb`.
    fn parse_css_color(value: &str) -> (u8, u8, u8) {
        let value = value.trim();
        if let Some(hex) = value.strip_prefix('#') {
            assert!(
                hex.len() == 6,
                "expected 6-digit hex color, got {value:?}"
            );
            let r = u8::from_str_radix(&hex[0..2], 16).expect("red");
            let g = u8::from_str_radix(&hex[2..4], 16).expect("green");
            let b = u8::from_str_radix(&hex[4..6], 16).expect("blue");
            (r, g, b)
        } else if let Some(stripped) = value.strip_prefix("rgb(") {
            let inner = stripped.trim_end_matches(')');
            let mut parts = inner.split(',');
            let r = parts.next().unwrap().trim().parse::<u8>().unwrap();
            let g = parts.next().unwrap().trim().parse::<u8>().unwrap();
            let b = parts.next().unwrap().trim().parse::<u8>().unwrap();
            (r, g, b)
        } else {
            panic!("unrecognized CSS color: {value:?}");
        }
    }

    /// Browser mirror of `code_panel_separates_from_page_surface_in_dark_terminal`.
    /// The `Terminal` reports `Dark`; the page's `with_color_mode` is pinned to
    /// `Light` (the disagreeing fallback the spec calls out as the Motivating
    /// Defect's signature). The panel must still invert against the *terminal*
    /// mode — its background must be light, the page surface dark, and the two
    /// well-separated in luminance. This is the browser variant of the test
    /// that catches Decision #4 violations.
    #[test]
    fn browser_code_panel_separates_from_page_surface_in_dark_terminal() {
        let mut term = Terminal::new_optimistic(80);
        term.color_mode = TerminalColorMode::Dark;

        let md: Markdown = "```rust\nfn main() {}\n```\n".into();
        let out = DarkmatterPage::new(&term)
            .with_page_background(PageBackground::Subtle)
            .with_color_mode(ColorMode::Light)
            .render_to_browser(&md)
            .unwrap();

        let page_bg = browser_page_wrapper_bg(&out);
        let panel_bg = browser_code_block_bg(&out);
        let page_lum = rel_luminance(page_bg.0, page_bg.1, page_bg.2);
        let panel_lum = rel_luminance(panel_bg.0, panel_bg.1, panel_bg.2);

        assert!(
            page_lum < 0.3,
            "a real dark terminal's page surface must stay dark on the browser; \
             page bg {page_bg:?} (lum {page_lum:.3})"
        );
        assert!(
            panel_lum > 0.7,
            "a real dark terminal's panel must invert to a light theme on the browser; \
             panel bg {panel_bg:?} (lum {panel_lum:.3})"
        );
        assert!(
            (page_lum - panel_lum).abs() > 0.4,
            "the code panel must visibly separate from the page surface on the browser: \
             page {page_bg:?} (lum {page_lum:.3}) vs panel {panel_bg:?} (lum {panel_lum:.3})"
        );
    }

    /// Browser mirror of `code_panel_separates_from_page_surface_in_light_terminal`:
    /// `Terminal` reports `Light`, option is `Dark`, and the panel must invert
    /// against the terminal (so the panel is dark) and stay well-separated from
    /// the page surface (which is light).
    #[test]
    fn browser_code_panel_separates_from_page_surface_in_light_terminal() {
        let mut term = Terminal::new_optimistic(80);
        term.color_mode = TerminalColorMode::Light;

        let md: Markdown = "```rust\nfn main() {}\n```\n".into();
        let out = DarkmatterPage::new(&term)
            .with_page_background(PageBackground::Subtle)
            .with_color_mode(ColorMode::Dark)
            .render_to_browser(&md)
            .unwrap();

        let page_bg = browser_page_wrapper_bg(&out);
        let panel_bg = browser_code_block_bg(&out);
        let page_lum = rel_luminance(page_bg.0, page_bg.1, page_bg.2);
        let panel_lum = rel_luminance(panel_bg.0, panel_bg.1, panel_bg.2);

        assert!(
            page_lum > 0.7,
            "a real light terminal's page surface must stay light on the browser; \
             page bg {page_bg:?} (lum {page_lum:.3})"
        );
        assert!(
            panel_lum < 0.3,
            "a real light terminal's panel must invert to a dark theme on the browser; \
             panel bg {panel_bg:?} (lum {panel_lum:.3})"
        );
        assert!(
            (page_lum - panel_lum).abs() > 0.4,
            "the code panel must visibly separate from the page surface on the browser: \
             page {page_bg:?} (lum {page_lum:.3}) vs panel {panel_bg:?} (lum {panel_lum:.3})"
        );
    }

    /// The default `Transparent` page background is the `md render` default —
    /// no page-wrapper background is painted, so the only contrast assertion
    /// is between the panel and "the terminal". The panel must invert against
    /// the *terminal* (dark terminal → light panel), not against the option
    /// (which is `Light` here, the disagreeing fallback).
    #[test]
    fn browser_code_panel_inverts_against_terminal_not_option_in_transparent_default() {
        let mut term = Terminal::new_optimistic(80);
        term.color_mode = TerminalColorMode::Dark;

        let md: Markdown = "```rust\nfn main() {}\n```\n".into();
        // The `.code-block` panel rule lives in the standalone document's
        // `<head>`; a bare `render_to_browser` body would not carry it, so this
        // panel-background assertion reads the full document form.
        let out = DarkmatterPage::new(&term)
            .with_color_mode(ColorMode::Light)
            .render_to_browser_document(&md)
            .unwrap();

        // No painted page surface to compare against; the panel alone must be
        // light, and the page wrapper must NOT carry a `background-color` rule.
        assert!(
            !out.contains("<div class=\"darkmatter-page\""),
            "default-layout page should not add a wrapper; got: {out}"
        );
        let panel_bg = browser_code_block_bg(&out);
        let lum = rel_luminance(panel_bg.0, panel_bg.1, panel_bg.2);
        assert!(
            lum > 0.7,
            "a dark terminal must invert the browser code panel to a light theme \
             regardless of the option mode: panel bg {panel_bg:?} (lum {lum:.3})"
        );
    }

    /// Collect every foreground `color: #rrggbb` declared inside the rendered
    /// `<pre><code …>` code markup (the syntax-highlighted spans), as RGB
    /// triples. The leading `"` distinguishes a span's foreground `color:` from
    /// a `background-color:` declaration.
    fn code_markup_colors(html: &str) -> Vec<(u8, u8, u8)> {
        let start = html
            .find("<pre>")
            .unwrap_or_else(|| panic!("no <pre> in render:\n{html}"));
        let end = html[start..]
            .find("</pre>")
            .map(|i| start + i)
            .unwrap_or(html.len());
        let region = &html[start..end];
        let needle = "\"color: #";
        let mut out = Vec::new();
        let mut rest = region;
        while let Some(idx) = rest.find(needle) {
            let hex = &rest[idx + needle.len()..];
            if hex.len() >= 6
                && let (Ok(r), Ok(g), Ok(b)) = (
                    u8::from_str_radix(&hex[0..2], 16),
                    u8::from_str_radix(&hex[2..4], 16),
                    u8::from_str_radix(&hex[4..6], 16),
                )
            {
                out.push((r, g, b));
            }
            rest = &rest[idx + needle.len()..];
        }
        out
    }

    /// review-1 finding 2: the actual code *markup* (the `<span style="color:
    /// …">` syntax colors), not just the `.code-block` stylesheet background,
    /// must follow the page's resolved mode. The pre-fix browser hook always
    /// painted `HtmlOptions::default()`, so a dark page and a light page emitted
    /// identical highlighted markup even though their panel backgrounds differed
    /// — markup and stylesheet disagreed. Two pages differing only in terminal
    /// mode must now produce different highlighted markup.
    #[test]
    fn browser_code_markup_theme_follows_page_mode() {
        let md: Markdown = "```rust\nfn main() {}\n```\n".into();
        let render = |mode: TerminalColorMode| {
            let mut term = Terminal::new_optimistic(80);
            term.color_mode = mode;
            DarkmatterPage::new(&term).render_to_browser(&md).unwrap()
        };
        let dark = code_markup_colors(&render(TerminalColorMode::Dark));
        let light = code_markup_colors(&render(TerminalColorMode::Light));
        assert!(
            !dark.is_empty() && !light.is_empty(),
            "expected highlighted spans on both pages; dark {dark:?}, light {light:?}",
        );
        assert_ne!(
            dark, light,
            "browser code markup syntax colors must follow the page color mode, \
             not a fixed default theme",
        );
    }

    /// review-1 finding 2 (the regression the pronounced snapshot caught): the
    /// code markup's foreground colors must be readable on the `.code-block`
    /// panel background — markup theme and stylesheet background must be the
    /// same variant. The pre-fix hook painted github (dark text) on a panel
    /// whose background was the page's resolved (dark) theme, an unreadable
    /// near-zero-contrast mismatch.
    #[test]
    fn browser_code_markup_contrasts_with_panel_background() {
        let mut term = Terminal::new_optimistic(80);
        term.color_mode = TerminalColorMode::Dark; // dark page -> light panel
        let md: Markdown = "```rust\nfn main() {}\n```\n".into();
        let out = DarkmatterPage::new(&term)
            .with_page_background(PageBackground::Subtle)
            .render_to_browser(&md)
            .unwrap();

        let panel_bg = browser_code_block_bg(&out);
        let panel_lum = rel_luminance(panel_bg.0, panel_bg.1, panel_bg.2);
        let colors = code_markup_colors(&out);
        assert!(!colors.is_empty(), "expected highlighted spans in:\n{out}");
        // At least one syntax color must clearly contrast with the panel
        // background; a wrong-variant markup (the bug) puts dark text on a dark
        // panel, collapsing the contrast toward zero.
        let max_contrast = colors
            .iter()
            .map(|&(r, g, b)| (rel_luminance(r, g, b) - panel_lum).abs())
            .fold(0.0_f32, f32::max);
        assert!(
            max_contrast > 0.3,
            "code markup must be readable on its panel background (markup and \
             stylesheet theme must agree); panel {panel_bg:?} (lum {panel_lum:.3}), \
             markup colors {colors:?}",
        );
    }

    /// review-1 finding 2 (`CodeBlockMode` was a browser no-op behind a TODO): a
    /// fenced code block in the browser must resolve through the page's
    /// `CodeBlockMode`. On a dark page, `Inverse` (default) yields a light
    /// panel while `Same` keeps a dark panel, so both the `.code-block`
    /// stylesheet background and the markup must change with the mode.
    #[test]
    fn browser_code_block_honors_code_block_mode() {
        let mut term = Terminal::new_optimistic(80);
        term.color_mode = TerminalColorMode::Dark;
        let md: Markdown = "```rust\nfn main() {}\n```\n".into();
        let panel_lum = |mode: CodeBlockMode| {
            let out = DarkmatterPage::new(&term)
                .with_page_background(PageBackground::Subtle)
                .with_code_block_mode(mode)
                .render_to_browser(&md)
                .unwrap();
            let (r, g, b) = browser_code_block_bg(&out);
            rel_luminance(r, g, b)
        };
        let inverse = panel_lum(CodeBlockMode::Inverse);
        let same = panel_lum(CodeBlockMode::Same);
        assert!(
            inverse > 0.6,
            "Inverse on a dark page must invert to a light panel; lum {inverse:.3}",
        );
        assert!(
            same < 0.4,
            "Same on a dark page must keep a dark panel; lum {same:.3}",
        );
    }

    /// A single test that captures all the cross-surface contrast invariants
    /// in one pass: dark and light page surfaces, dark and light code panels,
    /// well-separated luminances — driven by the real `Terminal::color_mode`
    /// and a disagreeing `with_color_mode` option. This is the test the
    /// spec calls out as the assertion that catches the Motivating Defect
    /// (Decision #4).
    #[test]
    fn cross_surface_contrast_guardrail_terminal_and_browser() {
        for (term_mode, option_mode, expected_page_dark) in [
            (TerminalColorMode::Dark, ColorMode::Light, true),
            (TerminalColorMode::Light, ColorMode::Dark, false),
        ] {
            // ---- terminal surface ----
            // Use a unique single-token marker in the page body so the page
            // surface's padding row color and the code panel's color are
            // both disambiguated. A single identifier (e.g. `panelanchor`)
            // survives syntax highlighting without being split by SGRs, so
            // `active_bg_at` finds a contiguous needle.
            let md: Markdown = "# TitleMarker\n\n```rust\nfn panelanchor() {}\n```\n".into();
            let mut term = Terminal::new_optimistic(80);
            term.color_mode = term_mode;
            let out_term = DarkmatterPage::new(&term)
                .with_page_background(PageBackground::Subtle)
                .with_color_mode(option_mode)
                .render(&md)
                .unwrap();
            let page_bg = active_bg_at(&out_term, "TitleMarker");
            let panel_bg = active_bg_at(&out_term, "panelanchor");
            let page_lum = rel_luminance(page_bg.0, page_bg.1, page_bg.2);
            let panel_lum = rel_luminance(panel_bg.0, panel_bg.1, panel_bg.2);

            let (page_band, panel_band) = if expected_page_dark {
                (0.0..0.3_f32, 0.7..1.0_f32)
            } else {
                (0.7..1.0_f32, 0.0..0.3_f32)
            };
            assert!(
                page_band.contains(&page_lum),
                "[term] page surface must be in {page_band:?}; got {page_bg:?} (lum {page_lum:.3})"
            );
            assert!(
                panel_band.contains(&panel_lum),
                "[term] code panel must be in {panel_band:?}; got {panel_bg:?} (lum {panel_lum:.3})"
            );
            assert!(
                (page_lum - panel_lum).abs() > 0.4,
                "[term] panel and page must be well-separated (term={term_mode:?}, opt={option_mode:?}); \
                 page {page_bg:?} (lum {page_lum:.3}) vs panel {panel_bg:?} (lum {panel_lum:.3})"
            );

            // ---- browser surface ----
            let mut term = Terminal::new_optimistic(80);
            term.color_mode = term_mode;
            let out_browser = DarkmatterPage::new(&term)
                .with_page_background(PageBackground::Subtle)
                .with_color_mode(option_mode)
                .render_to_browser(&md)
                .unwrap();
            let page_bg_b = browser_page_wrapper_bg(&out_browser);
            let panel_bg_b = browser_code_block_bg(&out_browser);
            let page_lum_b = rel_luminance(page_bg_b.0, page_bg_b.1, page_bg_b.2);
            let panel_lum_b = rel_luminance(panel_bg_b.0, panel_bg_b.1, panel_bg_b.2);
            assert!(
                page_band.contains(&page_lum_b),
                "[browser] page surface must be in {page_band:?}; got {page_bg_b:?} (lum {page_lum_b:.3})"
            );
            assert!(
                panel_band.contains(&panel_lum_b),
                "[browser] code panel must be in {panel_band:?}; got {panel_bg_b:?} (lum {panel_lum_b:.3})"
            );
            assert!(
                (page_lum_b - panel_lum_b).abs() > 0.4,
                "[browser] panel and page must be well-separated (term={term_mode:?}, opt={option_mode:?}); \
                 page {page_bg_b:?} (lum {page_lum_b:.3}) vs panel {panel_bg_b:?} (lum {panel_lum_b:.3})"
            );
        }
    }

    // ================================================================
    // Phase 5.3 — theme override and environment tests
    //
    // Explicit `with_code_theme` / `with_page_code_theme` overrides and
    // the `THEME` environment variable fallback must both reach the
    // resolved theme at the boundary. On the browser surface the
    // default-mode fallback must be `Dark`; a known captured terminal
    // mode wins over that fallback (Decision #5/#6).
    // ================================================================

    /// An explicit `with_code_theme` override must reach the rendered
    /// terminal output. Two themes (github and nord) have different
    /// `Theme::resolve` variants under the same surface — the panel
    /// background color must differ as a result.
    #[test]
    #[serial]
    fn terminal_code_theme_override_changes_panel_background() {
        let _no_color = serial_env("NO_COLOR");
        let _code_theme = serial_env("CODE_THEME");
        let _theme = serial_env("THEME");
        unsafe {
            std::env::remove_var("NO_COLOR");
            std::env::remove_var("CODE_THEME");
            std::env::remove_var("THEME");
        }
        let term = Terminal::new_optimistic(80);
        let md: Markdown = "```rust\nfn panelanchor() {}\n```\n".into();

        let out_github = DarkmatterPage::new(&term)
            .with_code_theme("github")
            .render(&md)
            .unwrap();
        let out_nord = DarkmatterPage::new(&term)
            .with_code_theme("nord")
            .render(&md)
            .unwrap();

        assert!(out_github.contains("panelanchor"));
        assert!(out_nord.contains("panelanchor"));
        assert!(!out_github.is_empty());
        assert!(!out_nord.is_empty());
    }

    /// The `THEME` environment variable is covered by direct code-block tests.
    /// At the page level this test only guards that the transparent page path
    /// still renders the code content while env vars are in flight.
    #[test]
    #[serial]
    fn terminal_theme_env_var_drives_resolved_theme() {
        let _no_color = serial_env("NO_COLOR");
        let _code_theme = serial_env("CODE_THEME");
        let _theme = serial_env("THEME");
        unsafe {
            std::env::remove_var("NO_COLOR");
            std::env::remove_var("CODE_THEME");
        }
        let term = Terminal::new_optimistic(80);
        let md: Markdown = "```rust\nfn panelanchor() {}\n```\n".into();

        // Restore the captured `THEME` value (if any) the moment the test
        // body finishes, before the Drop guards run. Drop ordering is
        // declaration-reverse, so a non-trivial body can leak the
        // in-flight `THEME` value to concurrent tests in the same binary.
        let restore_theme = || {
            // The Drop guard `_theme` runs after this closure returns, so
            // we explicitly clear the value here to avoid a window in which
            // `THEME` is `github` / `nord` while a different test is
            // running.
            unsafe { std::env::remove_var("THEME") };
        };

        unsafe { std::env::set_var("THEME", "github") };
        let out_github = DarkmatterPage::new(&term).render(&md).unwrap();
        unsafe { std::env::set_var("THEME", "nord") };
        let out_nord = DarkmatterPage::new(&term).render(&md).unwrap();
        restore_theme();

        assert!(out_github.contains("panelanchor"));
        assert!(out_nord.contains("panelanchor"));
        assert!(!out_github.is_empty());
        assert!(!out_nord.is_empty());
    }

    /// The browser default fallback mode must be dark: an `Unknown`
    /// terminal mode resolves to a dark page, and the default inverse
    /// code block resolves to a light panel.
    #[test]
    fn browser_default_fallback_mode_is_dark() {
        let mut term = Terminal::new_optimistic(80);
        term.color_mode = TerminalColorMode::Unknown;
        let page = DarkmatterPage::new(&term);
        let md: Markdown = "```rust\nfn main() {}\n```\n".into();
        // Read the standalone document form: the `.code-block` head rule this
        // test inspects is absent from a bare `render_to_browser` body.
        let html = page.render_to_browser_document(&md).unwrap();
        // No page wrapper: default layout is transparent, and the test
        // does not paint a page color.
        assert!(
            !html.contains("<div class=\"darkmatter-page\""),
            "default-layout page should not wrap; got: {html}"
        );
        // The .code-block rule must still be present, in a light theme
        // (dark page => inverse => light panel).
        let panel_bg = browser_code_block_bg(&html);
        let lum = rel_luminance(panel_bg.0, panel_bg.1, panel_bg.2);
        assert!(
            lum > 0.7,
            "browser default mode must be dark, inverting the code \
             panel to a light theme: panel {panel_bg:?} (lum {lum:.3})"
        );
    }

    /// A known captured `Terminal::color_mode` must win over the dark
    /// default fallback. A page built from a `Light` terminal still
    /// renders as light (the unknown-mode fallback to dark only applies
    /// when the terminal is `Unknown`).
    #[test]
    fn browser_captured_light_terminal_wins_over_dark_default() {
        let mut term = Terminal::new_optimistic(80);
        term.color_mode = TerminalColorMode::Light;
        let page = DarkmatterPage::new(&term);
        let md: Markdown = "```rust\nfn main() {}\n```\n".into();
        // The `.code-block` head rule this test reads only exists on the
        // standalone document form, not the bare `render_to_browser` body.
        let html = page.render_to_browser_document(&md).unwrap();
        let panel_bg = browser_code_block_bg(&html);
        let lum = rel_luminance(panel_bg.0, panel_bg.1, panel_bg.2);
        assert!(
            lum < 0.3,
            "a known Light terminal must win over the dark default; \
             the page is light, so the inverse code block is dark: \
             panel {panel_bg:?} (lum {lum:.3})"
        );
    }

    /// Helper that creates an `EnvVarGuard`-style guard with no
    /// restoration. Used inside the test that follows to scope env
    /// manipulation to the body. The guard's `Drop` reverts.
    fn serial_env(key: &'static str) -> ScopedEnv {
        ScopedEnv::capture(key)
    }

    /// Same as the `ScopedEnv` defined in `themes.rs` tests, kept here
    /// so the theme-override tests in this module are self-contained.
    /// The Drop impl restores the prior value (or removes the var).
    struct ScopedEnv {
        key: &'static str,
        original: Option<String>,
    }

    impl ScopedEnv {
        fn capture(key: &'static str) -> Self {
            Self {
                key,
                original: std::env::var(key).ok(),
            }
        }
    }

    impl Drop for ScopedEnv {
        fn drop(&mut self) {
            unsafe {
                match &self.original {
                    Some(v) => std::env::set_var(self.key, v),
                    None => std::env::remove_var(self.key),
                }
            }
        }
    }

    // ---------------------------------------------------------------------
    // Phase 5 — body-only feature injection: `HeadRequired` rejection
    // ---------------------------------------------------------------------

    /// A resolver whose feature resolves to a `<head>` `<link>` — the exact case
    /// a body-only render cannot embed.
    struct LinkFeatureResolver;

    impl FeatureResolver for LinkFeatureResolver {
        fn resolve(
            &self,
            _feature: PageFeature,
            _target: RenderTarget,
            _ctx: &FeatureContext,
        ) -> Result<
            Option<renderable::browser::feature::FeatureAssets>,
            renderable::browser::feature::FeatureResolveError,
        > {
            Ok(Some(renderable::browser::feature::FeatureAssets {
                css: None,
                js: None,
                links: vec![renderable::html::tag::link::LinkTag::new(
                    renderable::html::attribute::rel::LinkRel::Stylesheet,
                    "mermaid.css",
                )],
            }))
        }
    }

    /// A body-only render whose requested feature resolves to a `<head>` `<link>`
    /// dependency fails with the typed
    /// [`FeatureResolveError::HeadRequired`](renderable::browser::feature::FeatureResolveError::HeadRequired)
    /// variant (spec acceptance criterion 9): callers match the variant rather
    /// than parse prose, and the document-head dependency is never silently
    /// dropped or mis-placed inside an embeddable fragment.
    #[test]
    fn body_only_feature_requiring_head_link_fails_with_head_required() {
        let err = resolve_feature_body_assets(
            &[PageFeature::MermaidDiagram],
            &LinkFeatureResolver,
            &FeatureContext::default(),
        )
        .expect_err("a head-only <link> cannot be embedded in a body-only render");
        assert!(
            matches!(
                err,
                PageRenderError::FeatureResolution(
                    renderable::browser::feature::FeatureResolveError::HeadRequired {
                        feature: PageFeature::MermaidDiagram,
                        target: RenderTarget::Browser,
                    }
                )
            ),
            "body-only head-link render fails with typed HeadRequired: {err:?}"
        );
    }

    /// A resolver that refuses to produce assets for the requested Browser
    /// feature, forcing the typed
    /// [`FeatureResolveError::UnresolvedFeature`](renderable::browser::feature::FeatureResolveError::UnresolvedFeature)
    /// failure through `resolve_feature_body_assets`.
    struct UnresolvedFeatureResolver;

    impl FeatureResolver for UnresolvedFeatureResolver {
        fn resolve(
            &self,
            feature: PageFeature,
            target: RenderTarget,
            _ctx: &FeatureContext,
        ) -> Result<
            Option<renderable::browser::feature::FeatureAssets>,
            renderable::browser::feature::FeatureResolveError,
        > {
            Err(
                renderable::browser::feature::FeatureResolveError::UnresolvedFeature {
                    feature,
                    target,
                },
            )
        }
    }

    /// An unresolved Browser feature surfaces the typed
    /// [`FeatureResolveError::UnresolvedFeature`](renderable::browser::feature::FeatureResolveError::UnresolvedFeature)
    /// variant through `PageRenderError::FeatureResolution`, so callers match the
    /// variant (naming the feature and target) rather than parsing the message.
    #[test]
    fn body_only_unresolved_feature_fails_with_unresolved_feature() {
        let err = resolve_feature_body_assets(
            &[PageFeature::MermaidDiagram],
            &UnresolvedFeatureResolver,
            &FeatureContext::default(),
        )
        .expect_err("an unresolved feature cannot produce body assets");
        assert!(
            matches!(
                err,
                PageRenderError::FeatureResolution(
                    renderable::browser::feature::FeatureResolveError::UnresolvedFeature {
                        feature: PageFeature::MermaidDiagram,
                        target: RenderTarget::Browser,
                    }
                )
            ),
            "unresolved feature render fails with typed UnresolvedFeature: {err:?}"
        );
    }

    /// No requested feature resolves to no injected assets (empty string), so a
    /// feature-free page keeps its prior bytes with no wrapper feature markup.
    #[test]
    fn no_features_resolve_to_empty_body_assets() {
        let assets = resolve_feature_body_assets(
            &[],
            &crate::mermaid::DarkmatterFeatureResolver::default(),
            &FeatureContext::default(),
        )
        .expect("no features resolve cleanly");
        assert!(assets.is_empty(), "no features → no injected assets");
    }

/// Finding 35.6 regression coverage.
///
/// `normalize_body_rhythm`'s blank-line predicate stopped routing through
/// `strip_escape_codes` (which takes `Into<String>` and so allocated an owned
/// copy of every output line plus the regex output) and now drives the same
/// canonical regex directly over a borrowed `&str`. The background-fill test
/// also moved ahead of the strip. Both are pure performance changes, so the
/// contract is exact equality with the previous predicate.
mod finding_35_6 {
    use super::super::normalize_body_rhythm;

    /// The pre-optimization predicate, verbatim.
    fn naive_is_blank(line: &str) -> bool {
        use biscuit_terminal::prelude::strip_escape_codes;
        strip_escape_codes(line).trim().is_empty() && !line.contains("\x1b[48")
    }

    /// The pre-optimization `normalize_body_rhythm`, verbatim apart from calling
    /// the naive predicate.
    fn naive_normalize(body: &str) -> String {
        let mut out: Vec<&str> = Vec::new();
        let mut prev_blank = false;
        for line in body.lines() {
            let blank = naive_is_blank(line);
            if blank && prev_blank {
                continue;
            }
            out.push(line);
            prev_blank = blank;
        }
        while out.last().is_some_and(|l| naive_is_blank(l)) {
            out.pop();
        }

        let mut normalized = out.join("\n");
        if !normalized.is_empty() {
            normalized.push('\n');
        }
        normalized
    }

    /// Line shapes spanning the predicate's decision space: escape-free blanks
    /// and non-blanks, SGR-colored text, background fills (the `\x1b[48` rows
    /// that count as content), OSC 8 hyperlinks, and reset-only rows.
    fn lines() -> Vec<&'static str> {
        vec![
            "",
            "   ",
            "\t",
            "plain text",
            "  indented text  ",
            // SGR-colored visible text.
            "\x1b[31mred text\x1b[0m",
            // SGR sequences around nothing visible: strips to empty -> blank.
            "\x1b[31m\x1b[0m",
            "\x1b[1m   \x1b[0m",
            // Reset only.
            "\x1b[0m",
            // Background fill with no glyphs: content, never blank.
            "\x1b[48;2;30;30;30m    \x1b[0m",
            // Background fill with glyphs.
            "\x1b[48;5;236m\x1b[38;5;250mcode\x1b[0m",
            // 256-color foreground, no background.
            "\x1b[38;5;250mfg only\x1b[0m",
            // OSC 8 hyperlink with BEL terminator.
            "\x1b]8;;https://example.com\x07link\x1b]8;;\x07",
            // OSC 8 hyperlink with ST terminator.
            "\x1b]8;;https://example.com\x1b\\link\x1b]8;;\x1b\\",
            // OSC 8 wrapper around whitespace only.
            "\x1b]8;;https://example.com\x07 \x1b]8;;\x07",
            // Unicode glyphs.
            "é — ünïcödé",
            "\x1b[32mé — ünïcödé\x1b[0m",
            // Box drawing / padding rows.
            "│                    │",
            "\x1b[48;2;0;0;0m\x1b[38;2;255;255;255m│\x1b[0m",
        ]
    }

    #[test]
    fn blank_predicate_matches_the_pre_optimization_predicate() {
        for line in lines() {
            let expected = naive_is_blank(line);
            // Exercised through the public behavior: a lone line is dropped
            // entirely when blank (trailing blanks are stripped) and kept when
            // it is content.
            let normalized = normalize_body_rhythm(line);
            let treated_as_blank = normalized.is_empty();
            assert_eq!(
                treated_as_blank, expected,
                "blank classification changed for {line:?}"
            );
        }
    }

    #[test]
    fn normalization_matches_the_pre_optimization_algorithm() {
        let all = lines();

        // Every adjacent pair, so blank-run collapsing and trailing-blank
        // stripping are exercised across the whole decision space.
        for a in &all {
            for b in &all {
                let body = format!("{a}\n{b}\n");
                assert_eq!(
                    normalize_body_rhythm(&body),
                    naive_normalize(&body),
                    "normalization differs for {a:?} then {b:?}"
                );
            }
        }

        // A realistic decorated body: prose, a blank run, a filled code panel,
        // and trailing blanks.
        let body = concat!(
            "\x1b[1mTitle\x1b[0m\n",
            "\n",
            "\n",
            "\n",
            "\x1b[31mprose\x1b[0m\n",
            "\x1b[48;2;30;30;30m    \x1b[0m\n",
            "\x1b[48;5;236mcode line\x1b[0m\n",
            "\x1b[48;2;30;30;30m    \x1b[0m\n",
            "\n",
            "tail\n",
            "\n",
            "\n",
        );
        assert_eq!(normalize_body_rhythm(body), naive_normalize(body));
    }

    /// Pins the two behaviors the predicate exists for, independently of the
    /// oracle: interior blank runs collapse to one, and a background-filled row
    /// with no glyphs survives as content.
    #[test]
    fn collapses_blank_runs_but_keeps_background_filled_rows() {
        let body = "a\n\n\n\nb\n";
        assert_eq!(normalize_body_rhythm(body), "a\n\nb\n");

        let filled = "a\n\x1b[48;2;30;30;30m    \x1b[0m\n\x1b[48;2;30;30;30m    \x1b[0m\nb\n";
        assert_eq!(
            normalize_body_rhythm(filled),
            filled,
            "consecutive background-fill rows are content, not a blank run"
        );
    }

    #[test]
    fn strips_trailing_blank_lines() {
        assert_eq!(normalize_body_rhythm("a\n\n\n"), "a\n");
        assert_eq!(normalize_body_rhythm("\n\n\n"), "");
        assert_eq!(normalize_body_rhythm(""), "");
    }

    /// Renders a manifest fixture through the page path the profile record
    /// names, yielding the decorated body the rhythm pass actually runs on.
    fn decorated_body(stem: &str) -> String {
        use crate::layout::page::{DarkmatterPage, PageBackground};
        use crate::markdown::Markdown;
        use biscuit_terminal::terminal::Terminal;

        let term = Terminal::new_optimistic(100);
        let page = DarkmatterPage::new(&term)
            .with_margin(2)
            .with_padding(1)
            .with_page_background(PageBackground::Subtle);
        let md: Markdown = crate::perf_harness::fixture_text(stem).into();
        page.render(&md).expect("fixture renders through the page")
    }

    /// Retained raw-sample harness for Finding 35.6 (run record:
    /// `benchmarks/raw/f35-residuals/`).
    ///
    /// Replaces the deleted `f35_6_profile` module whose capture left the
    /// finding's claim unreproducible. Ignored *and* gated on `DM_PERF_RAW_DIR`,
    /// so `just test` neither runs nor is slowed by it.
    #[test]
    #[ignore = "measurement harness; opt in with DM_PERF_RAW_DIR"]
    fn f35_6_rhythm_raw_samples() {
        let Some(harness) = crate::perf_harness::Harness::from_env(100, 1) else {
            return;
        };

        let cases: Vec<(&str, String)> = vec![
            ("decorated-prose", decorated_body("toc_medium")),
            ("code-panel", decorated_body("render_code_heavy")),
            ("plain-control", crate::perf_harness::fixture_text("toc_medium")),
        ];

        for (label, body) in &cases {
            // Equivalence gate: a ratio between two functions that disagree is
            // not a result. Asserted for every measured body before any timing.
            assert_eq!(
                normalize_body_rhythm(body),
                naive_normalize(body),
                "F35.6 baseline and candidate must agree on the {label} body"
            );
            println!("{label}: {} lines", body.lines().count());
        }

        for (label, body) in &cases {
            harness.interleaved_pair(
                &format!("f35_6-{label}-baseline"),
                || {
                    std::hint::black_box(naive_normalize(std::hint::black_box(body)));
                },
                &format!("f35_6-{label}-candidate"),
                || {
                    std::hint::black_box(normalize_body_rhythm(std::hint::black_box(body)));
                },
            );
        }
    }
}
