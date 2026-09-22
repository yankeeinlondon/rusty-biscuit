use super::support::*;

#[test]
#[serial(level2_terminal)]
fn level2_file_links_directive_renders_styled_tree_in_real_terminal() {
    let Some((frame, dir)) = run_file_links_in_pane("file_links") else {
        return;
    };

    // Files discovered by `::file-links --dir docs/topics --depth 1`, relative
    // to the component root. The SAME list drives the visible-name checks and
    // the per-file OSC8 destination checks below, so every rendered file is
    // verified — not a representative subset that could leave regressions green.
    let expected_rel = [
        "alpha.md",
        "beta.md",
        "notes.txt",
        "report.pdf",
        "sheet.xlsx",
        "memo.docx",
        ".hidden.md",
        "ignored.md",
        "sub/nested.md",
        "sub/buried.md",
    ];

    // Visible hierarchy: the target directory, the nested subdirectory, and
    // every discovered document by its displayed (leaf) name.
    for token in ["topics", "sub"] {
        assert!(
            frame.plain.contains(token),
            "::file-links token {token:?} missing from real-terminal capture. plain:\n{}",
            frame.plain
        );
    }
    for rel in &expected_rel {
        let leaf = rel.rsplit('/').next().unwrap();
        assert!(
            frame.plain.contains(leaf),
            "::file-links file {leaf:?} missing from real-terminal capture. plain:\n{}",
            frame.plain
        );
    }

    // `.gitignore` is not a document extension, so it must not be in the tree.
    assert!(
        !frame.plain.contains(".gitignore"),
        ".gitignore should not be a tree entry. plain:\n{}",
        frame.plain
    );

    // The embedding marker must never surface as visible text.
    assert!(
        !frame.plain.contains("bt:render-tree"),
        "embedding marker leaked into visible capture. plain:\n{}",
        frame.plain
    );

    // Extension-specific Unicode glyphs distinguish each document kind (the
    // projection bakes Unicode icons; the bytes survive regardless of font).
    for (glyph, label) in &[
        ("📝", "txt"),
        ("📕", "pdf"),
        ("📗", "xls"),
        ("📘", "doc"),
    ] {
        assert!(
            frame.plain.contains(glyph),
            "expected {label} glyph {glyph:?} in capture. plain:\n{}",
            frame.plain
        );
    }

    // The root is a repository (`.git` present), so it renders the repository
    // icon (📦); the `sub` subdirectory renders the ordinary folder icon (📂).
    // Both appearing — each where expected — distinguishes the repo icon from
    // ordinary folder styling.
    assert!(
        frame.plain.contains("📦"),
        "expected repository root icon 📦 in capture. plain:\n{}",
        frame.plain
    );
    assert!(
        frame.plain.contains("📂"),
        "expected ordinary folder icon 📂 for the `sub` subdirectory. plain:\n{}",
        frame.plain
    );

    // Style assertions key off the OSC-stripped capture so a token is matched
    // as visible output, never inside a `file://` hyperlink URL.
    let styled = strip_osc(&frame.raw);

    // The gitignored entries are dim (SGR 2) on their own name — the root-level
    // rule (`ignored.md`) and a nested `.gitignore` below the component root
    // (`sub/buried.md`), which only dims correctly with directory-scoped Git
    // semantics. Asserting the run surrounding each name (not merely that *some*
    // dim exists) is what the dimmed root prefix alone could otherwise satisfy.
    for name in &["ignored.md", "buried.md"] {
        let attrs =
            active_sgr_params(&styled, name).unwrap_or_else(|| panic!("{name} missing from capture"));
        assert!(
            attrs.contains(&2),
            "gitignored `{name}` must carry the dim SGR on its own name; raw:\n{}",
            frame.raw
        );
    }

    // The dotfile `.hidden.md` is italic (SGR 3) on its own name.
    let hidden_attrs =
        active_sgr_params(&styled, ".hidden.md").expect(".hidden.md missing from capture");
    assert!(
        hidden_attrs.contains(&3),
        "dotfile `.hidden.md` must carry the italic SGR on its own name; raw:\n{}",
        frame.raw
    );

    // The highlighted target directory `topics` is bold (SGR 1) on its own
    // name. Keying off the `topics` token rules out the document's `# Root`
    // heading satisfying the assertion in its place.
    let topics_attrs = active_sgr_params(&styled, "topics").expect("topics missing from capture");
    assert!(
        topics_attrs.contains(&1) && !topics_attrs.contains(&2),
        "highlighted target `topics` must carry the bold SGR on its own name; raw:\n{}",
        frame.raw
    );

    // The boundary-relative root prefix renders dimmed (SGR 2) before the
    // highlighted target: the full `/docs/topics` root label is visible, and the
    // `/docs/` prefix run carries the dim SGR on its own (distinct from the bold,
    // non-dim `topics` asserted above).
    assert!(
        frame.plain.contains("/docs/topics"),
        "expected visible boundary-relative root `/docs/topics`; plain:\n{}",
        frame.plain
    );
    let prefix_attrs =
        active_sgr_params(&styled, "/docs/").expect("/docs/ root prefix missing from capture");
    assert!(
        prefix_attrs.contains(&2),
        "dimmed root prefix `/docs/` must carry the dim SGR on its own run; raw:\n{}",
        frame.raw
    );

    // Every rendered file carries its OWN OSC8 link to the correct canonical
    // `file://` destination — flat files, the dotfile, the gitignored file, and
    // both files inside the nested subtree. WezTerm re-emits hyperlink escapes in
    // its `--escapes` capture, so each full destination reaches the raw frame.
    assert!(
        frame.raw.contains("\u{1b}]8;;"),
        "expected OSC8 hyperlink introducer in the capture; raw:\n{}",
        frame.raw
    );
    let component_root = fs::canonicalize(dir.path().join("docs").join("topics"))
        .expect("canonicalize component root");
    for rel in &expected_rel {
        let want = format!("file://{}", component_root.join(rel).display());
        assert!(
            frame.raw.contains(&want),
            "expected per-file OSC8 destination {want:?} in capture; raw:\n{}",
            frame.raw
        );
    }
}
