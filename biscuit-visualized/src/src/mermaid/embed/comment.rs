//! The page's comments about the embedded font: which face, and why
//! ([`page_comment`]), and Inter's OFL attribution ([`ofl_attribution`]).
//!
//! A font's name is chosen by its vendor (or by whoever renamed a file), so
//! it is untrusted text. In a CSS context (`font-family`, `@font-face`) a name
//! is written with the post-process's `css_quoted`, the same spelling labels
//! lead with.

use super::faces::{ChoiceReason, EmbeddedFace, FamilyChoice};
use super::permission::{Permission, Refusal};

/// `name` with everything removed that could end, or confuse, the HTML
/// comment before the page's `<style>` or the CSS comment inside it:
///
/// - control characters (newlines included), `<`, and `>`;
/// - every `--` and `*/`, removed repeatedly until none remains, so `---` or
///   `*-*//` cannot leave one behind;
/// - a trailing `-`, which would form `--` with a closing `-->`.
///
/// The result is meant for display only; it may differ from the font's real
/// name.
pub(crate) fn comment_safe(name: &str) -> String {
    let mut cleaned: String = name
        .chars()
        .filter(|&c| !c.is_control() && c != '<' && c != '>')
        .collect();
    loop {
        let next = cleaned.replace("--", "").replace("*/", "");
        if next == cleaned {
            break;
        }
        cleaned = next;
    }
    while cleaned.ends_with('-') {
        cleaned.pop();
    }
    cleaned
}

/// The copyright line of the bundled Inter's `OFL.txt` (a test keeps the two
/// equal); the license file itself is not compiled in.
pub(crate) const INTER_COPYRIGHT: &str = "Copyright (c) 2016 The Inter Project Authors (https://github.com/rsms/inter)";

/// The one-line CSS comment that sits immediately before the bundled Inter's
/// first `@font-face` rule, and only Inter's: the OFL 1.1 notice travels in
/// the subset's `name` table, and this line makes it visible in the page.
pub(crate) fn ofl_attribution() -> String {
    format!("/* Inter: {}; SIL Open Font License 1.1 */", comment_safe(INTER_COPYRIGHT))
}

/// The HTML comment, placed immediately before the page's `<style>`, that
/// names the face embedded for `choice` and why it is that face. `unsubset`
/// is the embedded face that `fsType` forbids subsetting, when the page
/// embeds one, so the comment says the whole face is embedded.
pub(crate) fn page_comment(choice: &FamilyChoice, unsubset: Option<&EmbeddedFace>) -> String {
    let reason = match &choice.reason {
        ChoiceReason::Resolved => {
            let (regular, bold) = (&choice.regular, &choice.bold);
            if (regular.fs_type, regular.permission) == (bold.fs_type, bold.permission) {
                format!("the face the font chain resolved, {}", permission_text(regular))
            } else {
                format!(
                    "the face the font chain resolved, {} regular, {} bold",
                    permission_text(regular),
                    permission_text(bold)
                )
            }
        }
        ChoiceReason::MermaidFontInter => "selected by MERMAID_FONT=Inter".to_string(),
        ChoiceReason::Refused { face, refusal } => {
            let face = comment_safe(face);
            match refusal {
                Refusal::Restricted { fs_type } => {
                    format!("resolved face \"{face}\" is Restricted (fsType {fs_type:#06X})")
                }
                Refusal::BitmapOnly { fs_type } => {
                    format!("resolved face \"{face}\" allows bitmap embedding only (fsType {fs_type:#06X})")
                }
                Refusal::NoReadableOs2 => format!("resolved face \"{face}\" has no readable OS/2 table"),
                Refusal::OutlineFormat => {
                    format!("resolved face \"{face}\" has an outline format not embeddable as TrueType")
                }
            }
        }
        ChoiceReason::NoInstalledFace => "the font chain reached no installed face".to_string(),
    };
    let unsubset = unsubset.map_or_else(String::new, |face| {
        format!("; embedded unsubset because fsType {:#06X} forbids subsetting", face.fs_type)
    });
    format!(
        "<!-- mermaid font: \"{}\" embedded; {reason}{unsubset} -->",
        comment_safe(&choice.family)
    )
}

fn permission_text(face: &EmbeddedFace) -> String {
    let permission = match face.permission {
        Permission::Installable => "Installable",
        Permission::Editable => "Editable",
        Permission::PreviewAndPrint => "Preview & Print",
    };
    format!("fsType {:#06X} ({permission})", face.fs_type)
}
