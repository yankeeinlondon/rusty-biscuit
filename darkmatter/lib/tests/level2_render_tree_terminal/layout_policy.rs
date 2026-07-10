use super::support::*;

#[test]
#[serial(level2_terminal)]
fn level2_tree_table_cells_visible_in_real_terminal() {
    let body = "| Fruit | Quantity |\n|:------|---------:|\n| apples | 3 |\n| pears | 12 |\n";
    let Some((frame, _dir)) = run_in_pane(body, "table") else {
        return;
    };

    for token in &["Fruit", "Quantity", "apples", "pears", "12"] {
        assert!(
            frame.plain.contains(token),
            "table cell token {token:?} missing from real-terminal capture. plain:\n{}",
            frame.plain
        );
    }
}

#[test]
#[serial(level2_terminal)]
fn level2_unmatched_policy_matches_no_policy_color_in_real_terminal() {
    let body = "Lead prose paragraph.\n\n```rust\nfn demo() { let x = 1; }\n```\n";
    let Some((no_policy_frame, _d1)) =
        drive_pane(body, "no_policy_color", render_no_policy_page_to_tempfile)
    else {
        return;
    };
    let Some((unmatched_frame, _d2)) = drive_pane(
        body,
        "unmatched_policy_color",
        render_unmatched_policy_page_to_tempfile,
    ) else {
        return;
    };

    // The code body must survive both round-trips.
    for frame in [&no_policy_frame, &unmatched_frame] {
        assert!(
            frame.plain.contains("demo"),
            "code body token missing from real-terminal capture. plain:\n{}",
            frame.plain
        );
    }

    let no_policy_fg = foreground_color_set(&no_policy_frame);
    let unmatched_fg = foreground_color_set(&unmatched_frame);

    // Premise: the baseline render is actually colored, so the comparison is
    // meaningful rather than two empty sets.
    assert!(
        !no_policy_fg.is_empty(),
        "test premise: the no-policy code block must carry foreground color in the pane; \
         raw:\n{}",
        no_policy_frame.raw
    );
    // Parity: the unmatched policy must produce the same foreground colors.
    assert_eq!(
        no_policy_fg, unmatched_fg,
        "an unmatched policy must not change the real-terminal foreground colors of \
         unrelated content"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_matched_layout_policy_matches_no_policy_capabilities_in_real_terminal() {
    let Some(no_policy_frame) = drive_render_probe("no-policy") else {
        return;
    };
    let Some(matched_frame) = drive_render_probe("matched") else {
        return;
    };

    // Premise: the policy actually matched the table — the capped width plus
    // center alignment shifts the header row right, so the matched capture has
    // leading whitespace the (full-width) no-policy capture lacks. Center
    // alignment alone would not, since `Width::Auto` fills the pane; the
    // `max-width` cap is what makes centering observable. (A no-op policy would
    // make the parity checks below vacuous.)
    let table_header = |frame: &CapturedFrame| {
        frame
            .plain
            .lines()
            .find(|l| l.contains("A") && l.contains("B") && l.contains('\u{2502}'))
            .map(str::to_string)
            .unwrap_or_else(|| {
                panic!(
                    "table header row missing from real-terminal capture. plain:\n{}",
                    frame.plain
                )
            })
    };
    let no_policy_header = table_header(&no_policy_frame);
    let matched_header = table_header(&matched_frame);
    assert!(
        !no_policy_header.starts_with(' ') && matched_header.starts_with(' '),
        "test premise: the matched layout policy must center the table (leading whitespace) \
         while the no-policy render does not.\nno-policy header:{no_policy_header:?}\n\
         matched header:{matched_header:?}",
    );

    // The code body must survive both round-trips.
    for frame in [&no_policy_frame, &matched_frame] {
        assert!(
            frame.plain.contains("demo"),
            "code body token missing from real-terminal capture. plain:\n{}",
            frame.plain
        );
    }

    // Color axis: the unrelated code block's foreground colors must be identical.
    let no_policy_fg = foreground_color_set(&no_policy_frame);
    let matched_fg = foreground_color_set(&matched_frame);
    assert!(
        !no_policy_fg.is_empty(),
        "test premise: the no-policy code block must carry foreground color in the pane; \
         raw:\n{}",
        no_policy_frame.raw
    );
    assert_eq!(
        no_policy_fg, matched_fg,
        "a matched layout-only policy must not change the real-terminal foreground colors of \
         unrelated content"
    );

    // Hyperlink axis (discriminating, real-terminal): the page emitted a real
    // OSC8 hyperlink the pane honored, and the capture re-emits its URI. This is
    // non-vacuous evidence — it fails if the hyperlink metadata is absent from
    // the capture or the link degraded to plain text.
    let no_policy_links = osc8_openers(&no_policy_frame.raw);
    let matched_links = osc8_openers(&matched_frame.raw);
    assert!(
        no_policy_links.iter().any(|l| l.contains("https://example.com")),
        "test premise: the no-policy page must emit a real OSC8 hyperlink in the pane; \
         openers: {no_policy_links:?}\nraw:\n{}",
        no_policy_frame.raw
    );
    // The URL lives in OSC8 metadata, so the visible (plain) row carries the
    // label, not the URL — confirming the hyperlink is genuinely active.
    assert!(
        !no_policy_frame.plain.contains("example.com"),
        "the OSC8 URL must live in hyperlink metadata, not visible text. plain:\n{}",
        no_policy_frame.plain
    );
    // Parity: the matched layout-only policy must emit the byte-identical OSC8
    // opener set — it changes no hyperlink capability.
    assert_eq!(
        no_policy_links, matched_links,
        "a matched layout-only policy must not change the real-terminal OSC8 hyperlink behavior \
         of unrelated content",
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_zero_config_page_render_renders_in_real_terminal() {
    let body = "# Zero Config Page\n\nNo builder calls means the default-layout tree path.\n";
    let Some((frame, _dir)) = drive_pane(body, "zero_config_page", render_zero_config_page_to_tempfile)
    else {
        return;
    };

    for token in &["Zero Config Page", "No builder calls means the default-layout tree path."] {
        assert!(
            frame.plain.contains(token),
            "zero-config page token {token:?} missing from real-terminal capture. plain:\n{}",
            frame.plain
        );
    }
}

#[test]
#[serial(level2_terminal)]
fn level2_percent_page_frame_offset_and_width_in_real_terminal() {
    match wezterm_decision() {
        LevelDecision::Run => {}
        LevelDecision::Skip(msg) => {
            eprintln!("{msg}");
            return;
        }
        LevelDecision::Panic(msg) => panic!("{msg}"),
    }

    let mut guard = SHARED_HARNESS
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().unwrap();
    let cols = harness
        .pane_size()
        .map(|s| s.cols as usize)
        .unwrap_or(80)
        .max(40);

    // 25% left margin resolves against the full terminal width; 50% max-width
    // resolves against the post-margin content width.
    let expected_left = ((cols as f32) * 0.25).round() as usize;
    let content_base = cols - expected_left;
    let expected_max = ((content_base as f32) * 0.50).round() as usize;

    let sentinel = "Sentinel_pct_frame";
    let body = format!(
        "---\nstyle:\n    page:\n        left-margin: 25%\n        max-width: 50%\n---\n\n\
         {sentinel} lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod \
         tempor incididunt ut labore et dolore magna aliqua ut enim ad minim veniam.\n"
    );

    let md = Markdown::try_from_content(&body).expect("parse markdown with style frontmatter");
    let (style, _warnings) =
        darkmatter::style::from_frontmatter(md.frontmatter()).expect("parse style frontmatter");
    let term = Terminal::new_optimistic(cols as u32);
    let page = darkmatter::style::apply_page_style(
        DarkmatterPage::new(&term),
        &style,
        darkmatter::style::PageStyleOverrides::default(),
    )
    .expect("apply percentage page style");
    let rendered = page.render(&md).expect("decorated percent frame must render");

    let dir = tempdir().unwrap();
    let path = dir.path().join("percent_frame.ansi");
    fs::write(&path, rendered).unwrap();

    run_with_sentinel(harness, "clear");
    let frame = run_with_sentinel(harness, &format!("cat {}", path.display()));

    // The rendered paragraph is the sentinel row plus its wrapped continuations;
    // every content row carries the resolved left margin as leading spaces.
    let content_rows: Vec<&str> = frame
        .plain
        .lines()
        .skip_while(|l| !l.contains(sentinel))
        .take_while(|l| !l.trim().is_empty())
        .collect();

    assert!(
        content_rows.len() >= 2,
        "50% max-width ({expected_max} cols of {content_base}) must wrap the paragraph onto \
         multiple rows; got {} row(s). plain:\n{}",
        content_rows.len(),
        frame.plain
    );

    for (i, row) in content_rows.iter().enumerate() {
        let leading = row.chars().take_while(|c| *c == ' ').count();
        assert_eq!(
            leading, expected_left,
            "row +{i} must begin at the 25% left offset ({expected_left} cols of {cols}); \
             got {leading}. row: {row:?}"
        );
        let content_width = row.trim_end().chars().count() - leading;
        assert!(
            content_width <= expected_max,
            "row +{i} content width {content_width} exceeds the 50% cap ({expected_max} cols of \
             {content_base}). row: {row:?}"
        );
    }
}
