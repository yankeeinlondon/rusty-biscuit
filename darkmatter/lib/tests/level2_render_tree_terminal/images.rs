use super::support::*;

#[test]
#[serial(level2_terminal)]
fn level2_tree_hr_vector_mode_renders_glyph_not_image() {
    let body = "Lead paragraph.\n\n--- { style: waves }\n\nTrailing paragraph.\n";
    let Some((frame, _dir)) = run_in_pane_spanned_vector(body, "hr_vector_mode") else {
        return;
    };

    assert!(
        !frame.plain.contains("style: waves"),
        "raw HR markdown source leaked through; plain:\n{}",
        frame.plain
    );
    let is_waves_glyph = |c: char| c == '\u{224B}' || c == '~';
    let waves_rule_line = frame.plain.lines().find(|line| {
        let trimmed = line.trim();
        trimmed.chars().count() >= 10 && trimmed.chars().all(is_waves_glyph)
    });
    assert!(
        waves_rule_line.is_some(),
        "Vector mode must render the waves glyph (text tier), not an image; plain:\n{}",
        frame.plain
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_tree_rich_image_node_emits_protocol_and_renders_in_real_terminal() {
    match wezterm_decision() {
        LevelDecision::Run => {}
        LevelDecision::Skip(msg) => {
            eprintln!("{msg}");
            return;
        }
        LevelDecision::Panic(msg) => panic!("{msg}"),
    }

    // Fixture image + rendered bytes share one temp dir so the relative
    // `pic.png` resolves against `image_base_path` at render time.
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("pic.png"), TINY_PNG).unwrap();

    let source = SourceDescriptor::Virtual {
        name: "rich_image".into(),
    };
    let (doc, diags) = fold_markdown_to_document(source, "![cat](pic.png)\n");
    assert!(diags.is_empty(), "image fixture must fold cleanly: {diags:?}");

    // Rich tier on an iTerm2-capable TTY (WezTerm renders iTerm2 graphics).
    let mut term = Terminal::new_optimistic(120);
    term.image_support = ImageSupport::ITerm;
    term.is_tty = true;
    let mut context = TerminalRenderContext::from_terminal(&term);
    context.graphics_mode = renderable::tree::GraphicsMode::Rich;
    context.image_base_path = Some(dir.path().to_path_buf());
    let opts = TerminalRenderOptions {
        context,
        strictness: RenderStrictness::Warn,
        code_renderer: Some(Rc::new(TerminalCodeRenderer::new())),
    };
    let rendered = render_terminal_document(&doc, &opts).expect("tree terminal render");

    // In-process proof the production tree path emitted the inline image
    // protocol (iTerm2 OSC 1337), not the alt-text fallback.
    assert!(
        rendered.output.contains("\u{1b}]1337;File="),
        "Rich image node must emit the iTerm2 image protocol; rendered bytes:\n{:?}",
        rendered.output,
    );
    assert!(
        !rendered.output.contains("[cat]"),
        "successful image render must not emit the bracketed alt-text fallback",
    );

    let path = dir.path().join("rich_image.ansi");
    fs::write(&path, rendered.output).unwrap();

    let mut guard = SHARED_HARNESS
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().unwrap();
    run_with_sentinel(harness, "clear");
    let frame = run_with_sentinel(harness, &format!("cat {}", path.display()));

    // Anti-regression only: the real terminal must not surface the `[cat]`
    // alt-text fallback. This does NOT prove the image was painted — the
    // harness cannot inspect rendered pixels (see the verification-scope note
    // on this fn). A dropped/ignored payload would also pass this assertion.
    assert!(
        !frame.plain.contains("[cat]"),
        "real terminal showed the alt-text fallback instead of consuming the image. plain:\n{}",
        frame.plain,
    );
}

#[test]
fn pixel_classification_distinguishes_magenta_from_black() {
    const MAGENTA: [u8; 3] = [255, 0, 255];
    let dir = tempdir().unwrap();

    let magenta_path = dir.path().join("m.png");
    write_solid_png(&magenta_path, 64, MAGENTA);
    let (near, non_black, total) = classify_pixels(&fs::read(&magenta_path).unwrap(), MAGENTA, 60);
    assert_eq!(total, 64 * 64);
    assert_eq!(near, total, "every magenta pixel must classify as near-target");
    assert_eq!(non_black, total, "magenta is not black");

    let black_path = dir.path().join("b.png");
    write_solid_png(&black_path, 64, [0, 0, 0]);
    let (near, non_black, _) = classify_pixels(&fs::read(&black_path).unwrap(), MAGENTA, 60);
    assert_eq!(near, 0, "black has no magenta");
    assert_eq!(non_black, 0, "black capture must read as blocked/empty");
}

// The pixel-readback companion (`level3_rich_image_node_paints_distinctive_pixels`)
// lives in `darkmatter/lib/tests/level3_image_painting.rs`. It calls
// `WezTermHarness::capture_window_png`, which raises the WezTerm window to
// the foreground and invokes `screencapture` — a focus-stealing, OS-level
// operation that the `biscuit-test-harness` contract reserves for L3. Keeping
// it in the L2 suite broke the "L2 never steals focus" invariant.
