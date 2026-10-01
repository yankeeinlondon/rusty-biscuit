use super::*;

fn names(parts: &[&str]) -> Vec<OsString> {
    parts.iter().map(OsString::from).collect()
}

fn render(lead: Lead<'_>, parts: &[&str]) -> Result<String, TextRejection> {
    render_reference(lead, &names(parts))
}

// ---- spelling per lead --------------------------------------------------

#[test]
fn each_lead_spells_its_reference_form() {
    let cases: &[(Lead<'_>, &[&str], &str)] = &[
        (Lead::Relative { parent_hops: 0 }, &["guide.md"], "./guide.md"),
        (Lead::Relative { parent_hops: 2 }, &["assets", "logo.png"], "../../assets/logo.png"),
        (Lead::Relative { parent_hops: 0 }, &[], "."),
        (Lead::Relative { parent_hops: 2 }, &[], "../.."),
        (Lead::Bare, &["docs", "x.md"], "docs/x.md"),
        (Lead::Home, &["notes", "x.md"], "~/notes/x.md"),
        (Lead::Home, &[], "~"),
        (Lead::RepositoryRoot, &["docs", "x.md"], "&docs/x.md"),
        (Lead::RepositoryScoped, &["docs", "x.md"], "^docs/x.md"),
        (Lead::Magic, &["docs", "x.md"], "@docs/x.md"),
        (Lead::Env("NOTES"), &["x.md"], "{{NOTES}}/x.md"),
        (Lead::Env("NOTES"), &[], "{{NOTES}}"),
    ];
    for (lead, parts, expected) in cases {
        assert_eq!(render(*lead, parts).as_deref(), Ok(*expected), "{lead:?} {parts:?}");
    }
}

/// Every rendered spelling must round-trip through the parser as the form it
/// was rendered for; the dependent output is the parsed kind, not the text.
#[test]
fn rendered_text_parses_back_as_the_intended_kind() {
    for (lead, kind) in [
        (Lead::Relative { parent_hops: 1 }, FileReferenceKind::ExplicitRelative),
        (Lead::Bare, FileReferenceKind::ImplicitRelative),
        (Lead::Home, FileReferenceKind::Home),
        (Lead::RepositoryRoot, FileReferenceKind::RepositoryRoot),
        (Lead::Magic, FileReferenceKind::Magic),
    ] {
        let text = render(lead, &["a", "b.md"]).unwrap();
        let parsed = crate::FileReference::new(&text).unwrap();
        assert_eq!(parsed.class().kind, kind, "{text}");
    }
}

// ---- grammar ------------------------------------------------------------

#[test]
fn a_leading_sigil_in_a_bare_name_is_rejected_and_dot_slash_protects_it() {
    for name in ["@notes.md", "&notes.md", "^notes.md", "~notes.md", "%notes.md", "!notes.md"] {
        assert_eq!(render(Lead::Bare, &[name]), Err(TextRejection::GrammarMismatch), "{name}");
        assert_eq!(
            render(Lead::Relative { parent_hops: 0 }, &[name]),
            Ok(format!("./{name}")),
            "{name}"
        );
    }
}

/// `./` protects a leading sigil but not interpolation inside a name.
#[test]
fn interpolation_in_a_literal_name_is_rejected_under_every_lead() {
    for name in ["{{HOME}}.md", "a{{X}}b.md", "{{lower}}.md", "open{{.md"] {
        for lead in [
            Lead::Relative { parent_hops: 0 },
            Lead::Relative { parent_hops: 1 },
            Lead::Bare,
            Lead::Home,
            Lead::RepositoryRoot,
            Lead::Env("NOTES"),
        ] {
            assert_eq!(
                render(lead, &["dir", name]),
                Err(TextRejection::GrammarMismatch),
                "{lead:?} {name}"
            );
        }
    }
    // A lone closing pair is not grammar.
    assert_eq!(render(Lead::Bare, &["a}}b.md"]).as_deref(), Ok("a}}b.md"));
}

#[test]
fn a_sigil_lead_with_no_names_is_rejected() {
    for lead in [Lead::RepositoryRoot, Lead::RepositoryScoped, Lead::Magic, Lead::Bare] {
        assert_eq!(render(lead, &[]), Err(TextRejection::GrammarMismatch), "{lead:?}");
    }
}

/// A `:` is legal in a Unix name, where the parser then reads the text as a
/// scheme, vault, or drive; Win32 forbids it in a name outright.
#[test]
fn a_colon_name_is_rejected_on_every_host() {
    let expected = if cfg!(windows) {
        TextRejection::ChangesComponents
    } else {
        TextRejection::GrammarMismatch
    };
    assert_eq!(render(Lead::RepositoryRoot, &["C:x"]), Err(expected));
    assert_eq!(render(Lead::Bare, &["vault:notes.md"]), Err(expected));
    assert_eq!(render(Lead::Bare, &["mailto:x.md"]), Err(expected));
    if cfg!(not(windows)) {
        assert_eq!(
            render(Lead::Relative { parent_hops: 0 }, &["vault:notes.md"]).as_deref(),
            Ok("./vault:notes.md")
        );
    }
}

// ---- component faithfulness --------------------------------------------

/// A literal `.` or `..` name (possible below a Windows verbatim prefix) would
/// be read as navigation once written as text.
#[test]
fn literal_dot_names_are_rejected() {
    for parts in [&[".."][..], &["a", ".."], &["."], &["a", ".", "b"]] {
        assert_eq!(
            render(Lead::Relative { parent_hops: 0 }, parts),
            Err(TextRejection::ChangesComponents),
            "{parts:?}"
        );
    }
}

#[cfg(unix)]
#[test]
fn a_unix_backslash_name_is_rejected_rather_than_split() {
    assert_eq!(
        render(Lead::Relative { parent_hops: 0 }, &[r"my\report.md"]),
        Err(TextRejection::ChangesComponents)
    );
    assert_eq!(
        render_absolute(Path::new(r"/repo/my\report.md")),
        Err(TextRejection::ChangesComponents)
    );
}

#[cfg(unix)]
#[test]
fn non_unicode_names_are_unrenderable_before_any_other_check() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let bad = OsStr::from_bytes(b"bad\xFF.md").to_os_string();
    assert_eq!(
        render_reference(Lead::Relative { parent_hops: 0 }, std::slice::from_ref(&bad)),
        Err(TextRejection::Unrenderable)
    );
    // Even next to a name that would otherwise be a grammar rejection.
    assert_eq!(
        render_reference(Lead::Bare, &[OsString::from("@x"), bad]),
        Err(TextRejection::Unrenderable)
    );
    assert_eq!(
        render_absolute(Path::new(OsStr::from_bytes(b"/repo/bad\xFF.md"))),
        Err(TextRejection::Unrenderable)
    );
}

#[test]
fn names_that_change_without_a_verbatim_prefix_are_detected() {
    for name in ["CON", "con.txt", "con.. .txt", "LPT9", "trailing.", "trailing ", "a:b", "a*b", "a?b", "a|b", "a\"b", "a<b", "a>b", "tab\tname", ".", ".."] {
        assert!(!survives_without_verbatim_prefix(name), "{name}");
    }
    for name in ["console.md", "CONFIG", "a.b", ".hidden", "logo.png", "COM10"] {
        assert!(survives_without_verbatim_prefix(name), "{name}");
    }
}

/// Windows and `dunce` count name length in UTF-16 units, not `char`s.
#[test]
fn name_length_is_measured_in_utf16_units() {
    assert!(survives_without_verbatim_prefix(&"a".repeat(255)));
    assert!(!survives_without_verbatim_prefix(&"a".repeat(256)));
    assert!(survives_without_verbatim_prefix(&"😀".repeat(127)));
    assert!(!survives_without_verbatim_prefix(&"😀".repeat(128)));
}

#[cfg(windows)]
#[test]
fn a_windows_name_that_changes_meaning_is_rejected() {
    for name in ["CON", "trailing."] {
        assert_eq!(
            render(Lead::Relative { parent_hops: 0 }, &["dir", name]),
            Err(TextRejection::ChangesComponents),
            "{name}"
        );
    }
}

// ---- absolute -----------------------------------------------------------

#[cfg(unix)]
#[test]
fn an_absolute_unix_path_keeps_its_spelling() {
    assert_eq!(render_absolute(Path::new("/repo/docs/x.md")).as_deref(), Ok("/repo/docs/x.md"));
}

#[cfg(unix)]
#[test]
fn interpolation_in_an_absolute_name_is_rejected() {
    assert_eq!(
        render_absolute(Path::new("/repo/{{X}}.md")),
        Err(TextRejection::GrammarMismatch)
    );
    assert_eq!(
        render_absolute(Path::new("/repo/{{x.md")),
        Err(TextRejection::GrammarMismatch)
    );
}

#[test]
fn a_relative_input_is_not_an_absolute_spelling() {
    assert_eq!(render_absolute(Path::new("docs/x.md")), Err(TextRejection::GrammarMismatch));
}

#[cfg(windows)]
#[test]
fn windows_absolute_spellings() {
    assert_eq!(render_absolute(Path::new(r"C:\repo\x.md")).as_deref(), Ok("C:/repo/x.md"));
    assert_eq!(render_absolute(Path::new(r"\\?\C:\repo\x.md")).as_deref(), Ok("C:/repo/x.md"));
    // A legacy UNC path keeps its faithful native spelling.
    assert_eq!(
        render_absolute(Path::new(r"\\server\share\x.md")).as_deref(),
        Ok(r"\\server\share\x.md")
    );
    for path in [r"\\?\UNC\server\share\x.md", r"\\.\COM1", r"\\?\C:\repo\CON"] {
        assert_eq!(
            render_absolute(Path::new(path)),
            Err(TextRejection::NoPortableSpelling),
            "{path}"
        );
    }
}
