//! The page's comments about the embedded font (D14, D15; AC-E14, AC-E15),
//! and font names in the HTML comment, the CSS comment, and CSS strings.

use resvg::usvg::fontdb::ID;

use crate::mermaid::embed::comment::{INTER_COPYRIGHT, comment_safe, ofl_attribution, page_comment};
use crate::mermaid::embed::faces::{ChoiceReason, EmbeddedFace, FamilyChoice};
use crate::mermaid::embed::permission::{Permission, Refusal};
use crate::mermaid::postprocess::css_quoted;

/// Whether `text`, placed in an HTML comment as `<!-- … "text" … -->` and in
/// a CSS comment as `/* … text … */`, leaves both comments intact.
fn assert_comment_safe(original: &str, text: &str) {
    for forbidden in ["--", ">", "<", "*/", "\n", "\r"] {
        assert!(!text.contains(forbidden), "{original:?} -> {text:?} contains {forbidden:?}");
    }
    assert!(!text.chars().any(char::is_control), "{original:?} -> {text:?} has a control");
    assert!(!text.ends_with('-'), "{original:?} -> {text:?} ends with '-'");
    let html = format!("<!-- mermaid font: \"{text}\" embedded -->");
    assert_eq!(html.matches("-->").count(), 1, "{html}");
    assert!(html.ends_with("-->"), "{html}");
    let css = format!("/* {text} */");
    assert_eq!(css.matches("*/").count(), 1, "{css}");
}

#[test]
fn crafted_names_cannot_end_either_comment() {
    let cases = [
        ("a--b", "ab"),
        ("a-->b", "ab"),
        ("---", ""),
        ("a---b", "a-b"),
        ("*/ x", " x"),
        ("**//", ""),
        ("*-*//", "*-/"),
        ("-*/-", ""),
        ("Name\"Quote", "Name\"Quote"),
        ("Back\\slash", "Back\\slash"),
        ("line\nbreak", "linebreak"),
        ("<!-- inner -->", "! inner "),
        ("ends with -", "ends with "),
        ("--!>", "!"),
        ("Trebuchet MS", "Trebuchet MS"),
    ];
    for (name, expected) in cases {
        let cleaned = comment_safe(name);
        assert_eq!(cleaned, expected, "{name:?}");
        assert_comment_safe(name, &cleaned);
    }
}

#[test]
fn generated_names_cannot_end_either_comment() {
    const ALPHABET: [char; 12] = ['-', '-', '>', '<', '*', '/', '!', 'a', ' ', '\n', '\u{7}', '"'];
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for _ in 0..5_000 {
        let len = (next() % 12) as usize;
        let name: String = (0..len)
            .map(|_| ALPHABET[(next() % ALPHABET.len() as u64) as usize])
            .collect();
        assert_comment_safe(&name, &comment_safe(&name));
    }
}

#[test]
fn css_strings_escape_quotes_backslashes_and_markup() {
    for (name, expected) in [
        ("Arial", "'Arial'"),
        ("a\"b", "'a\\22 b'"),
        ("a'b", "'a\\27 b'"),
        ("a\\b", "'a\\5c b'"),
        ("</style>", "'\\3c /style\\3e '"),
        ("line\nbreak", "'line\\a break'"),
    ] {
        let quoted = css_quoted(name);
        assert_eq!(quoted, expected, "{name:?}");
        // One CSS string: only the two delimiting quotes are unescaped.
        assert_eq!(quoted.matches('\'').count(), 2, "{quoted}");
        assert!(!quoted.contains('"') && !quoted.contains('<') && !quoted.contains('\n'));
    }
}

fn face(permission: Permission, fs_type: u16) -> EmbeddedFace {
    EmbeddedFace {
        id: ID::dummy(),
        permission: Ok(permission),
        fs_type,
        unsubset: fs_type & 0x0100 != 0,
    }
}

fn choice(family: &str, regular: EmbeddedFace, bold: EmbeddedFace, reason: ChoiceReason) -> FamilyChoice {
    FamilyChoice {
        requested: "\"trebuchet ms\", verdana, arial, sans-serif".to_string(),
        family: family.to_string(),
        bundled_inter: family == "Inter",
        regular,
        bold,
        reason,
    }
}

fn inter(reason: ChoiceReason) -> FamilyChoice {
    let installable = face(Permission::Installable, 0);
    choice("Inter", installable, installable, reason)
}

fn refused(refusal: Refusal) -> String {
    page_comment(
        &inter(ChoiceReason::Refused {
            face: "Arial".to_string(),
            refusal,
        }),
        None,
    )
}

#[test]
fn the_comment_names_a_resolved_face_and_its_permission() {
    for (permission, fs_type, text) in [
        (Permission::Installable, 0x0000, "fsType 0x0000 (Installable)"),
        (Permission::Editable, 0x0008, "fsType 0x0008 (Editable)"),
        (Permission::PreviewAndPrint, 0x0004, "fsType 0x0004 (Preview & Print)"),
    ] {
        let face = face(permission, fs_type);
        assert_eq!(
            page_comment(&choice("Arial", face, face, ChoiceReason::Resolved), None),
            format!("<!-- mermaid font: \"Arial\" embedded; the face the font chain resolved, {text} -->")
        );
    }
    let mixed = choice(
        "Arial",
        face(Permission::Installable, 0x0000),
        face(Permission::Editable, 0x0008),
        ChoiceReason::Resolved,
    );
    assert_eq!(
        page_comment(&mixed, None),
        "<!-- mermaid font: \"Arial\" embedded; the face the font chain resolved, \
         fsType 0x0000 (Installable) regular, fsType 0x0008 (Editable) bold -->"
    );
    // A refused weight is never drawn, so only the permitted one is named.
    let restricted = EmbeddedFace {
        permission: Err(Refusal::Restricted { fs_type: 0x0002 }),
        ..face(Permission::Installable, 0x0002)
    };
    for (regular, bold) in [
        (face(Permission::Editable, 0x0008), restricted),
        (restricted, face(Permission::Editable, 0x0008)),
    ] {
        assert_eq!(
            page_comment(&choice("Arial", regular, bold, ChoiceReason::Resolved), None),
            "<!-- mermaid font: \"Arial\" embedded; the face the font chain resolved, fsType 0x0008 (Editable) -->"
        );
    }
}

#[test]
fn the_comment_names_inter_and_why_for_every_substitution() {
    assert_eq!(
        page_comment(&inter(ChoiceReason::MermaidFontInter), None),
        "<!-- mermaid font: \"Inter\" embedded; selected by MERMAID_FONT=Inter -->"
    );
    // The spec's own example, exactly.
    assert_eq!(
        refused(Refusal::Restricted { fs_type: 0x0002 }),
        "<!-- mermaid font: \"Inter\" embedded; resolved face \"Arial\" is Restricted (fsType 0x0002) -->"
    );
    assert_eq!(
        refused(Refusal::BitmapOnly { fs_type: 0x0200 }),
        "<!-- mermaid font: \"Inter\" embedded; resolved face \"Arial\" allows bitmap embedding only (fsType 0x0200) -->"
    );
    assert_eq!(
        refused(Refusal::NoReadableOs2),
        "<!-- mermaid font: \"Inter\" embedded; resolved face \"Arial\" has no readable OS/2 table -->"
    );
    assert_eq!(
        refused(Refusal::OutlineFormat),
        "<!-- mermaid font: \"Inter\" embedded; resolved face \"Arial\" has an outline format not embeddable as TrueType -->"
    );
    assert_eq!(
        page_comment(&inter(ChoiceReason::NoInstalledFace), None),
        "<!-- mermaid font: \"Inter\" embedded; the font chain reached no installed face -->"
    );
}

#[test]
fn the_comment_says_when_the_face_is_embedded_unsubset() {
    let whole = face(Permission::Editable, 0x0108);
    let resolved = choice("Arial", whole, whole, ChoiceReason::Resolved);
    assert_eq!(
        page_comment(&resolved, Some(&whole)),
        "<!-- mermaid font: \"Arial\" embedded; the face the font chain resolved, fsType 0x0108 (Editable); \
         embedded unsubset because fsType 0x0108 forbids subsetting -->"
    );
}

#[test]
fn crafted_family_and_face_names_cannot_end_the_page_comment() {
    let crafted = "Evil--> <script>alert(1)</script> <!-- -";
    let installable = face(Permission::Installable, 0);
    let comments = [
        page_comment(&choice(crafted, installable, installable, ChoiceReason::Resolved), None),
        page_comment(
            &inter(ChoiceReason::Refused {
                face: crafted.to_string(),
                refusal: Refusal::Restricted { fs_type: 2 },
            }),
            Some(&installable),
        ),
    ];
    for comment in comments {
        assert!(comment.starts_with("<!-- mermaid font: "), "{comment}");
        assert_eq!(comment.matches("-->").count(), 1, "{comment}");
        assert!(comment.ends_with(" -->"), "{comment}");
        assert_eq!(comment.matches("<!--").count(), 1, "{comment}");
        assert!(!comment.contains("<script"), "{comment}");
    }
}

#[test]
fn the_ofl_attribution_is_one_css_comment_naming_inters_copyright() {
    assert_eq!(
        ofl_attribution(),
        "/* Inter: Copyright (c) 2016 The Inter Project Authors (https://github.com/rsms/inter); \
         SIL Open Font License 1.1 */"
    );
    assert_eq!(ofl_attribution().matches("*/").count(), 1);
    assert!(!ofl_attribution().contains('\n'));
}

#[test]
fn the_copyright_line_is_the_bundled_ofl_txt_first_line() {
    let ofl = include_str!("../../assets/fonts/inter/OFL.txt");
    assert_eq!(ofl.lines().next(), Some(INTER_COPYRIGHT));
    // D8: no Reserved Font Name, so the embedded family is declared `Inter`.
    assert!(!ofl.lines().next().unwrap_or_default().contains("Reserved Font Name"));
}
