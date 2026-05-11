use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

use super::super::choose::{HotkeyDisplayMode, HotkeySpec};

/// 256-color palette index for the held Ctrl badge background
/// (bright orange; reads on both dark and light terminals).
pub(super) const CTRL_BADGE_BG: Color = Color::Indexed(208);
/// 256-color palette index for the held Alt badge background (bright
/// yellow; reads on both dark and light terminals).
pub(super) const ALT_BADGE_BG: Color = Color::Indexed(220);
/// Darker shade of the Ctrl orange used when this badge is *not* the
/// currently-emphasised modifier. The spec asks for badges to stay
/// visible across both states; using a darker shade keeps the family
/// colour identifiable while making the held state visually dominant
/// without relying on the unreliable `Modifier::DIM` SGR.
pub(super) const CTRL_BADGE_BG_DIM: Color = Color::Indexed(166);
/// Darker shade of the Alt yellow for the not-held state. See
/// [`CTRL_BADGE_BG_DIM`] for rationale.
pub(super) const ALT_BADGE_BG_DIM: Color = Color::Indexed(178);
/// Foreground colour for hotkey badge text on the **orange** Ctrl
/// background. Black reads cleanly on the bright orange across both
/// held and dim shades — white-on-orange has marginal contrast on
/// many terminal palettes.
pub(super) const BADGE_FG_ON_ORANGE: Color = Color::Black;
/// Foreground colour for hotkey badge text on the **yellow** Alt
/// background. Black reads cleanly on yellow; white-on-yellow renders
/// as a near-illegible blur on most terminals.
pub(super) const BADGE_FG_ON_YELLOW: Color = Color::Black;

/// Builds a renderable string for a hotkey badge.
///
/// For example, `HotkeySpec::Ctrl('r')` renders as `^R`. The compact
/// `^X` / `⌥X` glyphs were chosen so badges fit inside narrow rows
/// without breaking option alignment.
fn badge_text(hotkey: HotkeySpec) -> String {
    match hotkey {
        HotkeySpec::Ctrl(c) => format!("^{}", c.to_ascii_uppercase()),
        HotkeySpec::Alt(c) => format!("⌥{}", c.to_ascii_uppercase()),
    }
}

/// Builds a styled span for a hotkey badge.
///
/// Per spec:
/// > those attached to CTRL will have an orange background the hotkey will
/// > be bold faced; those attached to ALT will have a yellow background
/// > and the hotkey will be dim/light font
///
/// The background colour (orange for Ctrl, yellow for Alt) stays in
/// **both** states — it's how the user reads the family of a hotkey
/// at a glance. The difference between held and not-held is the font
/// weight:
///
/// - **Held** (this badge's modifier is what the user is emphasising):
///   bright family BG + bold white FG.
/// - **Not held**: a *darker* shade of the same family colour for the
///   BG, with regular (non-bold) white FG. The darker BG plus removal
///   of bold is the visual cue that this isn't the active modifier.
///   We deliberately do NOT use `Modifier::DIM`: it renders
///   inconsistently across terminals (often invisible in WezTerm's
///   default theme), which made the original DIM-on-bright treatment
///   look identical to the held state.
///
/// `display == Hidden` → no badge is produced.
pub(super) fn badge_span(hotkey: HotkeySpec, display: HotkeyDisplayMode) -> Option<Span<'static>> {
    if display == HotkeyDisplayMode::Hidden {
        return None;
    }
    let (held_bg, dim_bg, fg, this_held) = match hotkey {
        HotkeySpec::Ctrl(_) => (
            CTRL_BADGE_BG,
            CTRL_BADGE_BG_DIM,
            BADGE_FG_ON_ORANGE,
            display == HotkeyDisplayMode::CtrlHeld,
        ),
        HotkeySpec::Alt(_) => (
            ALT_BADGE_BG,
            ALT_BADGE_BG_DIM,
            BADGE_FG_ON_YELLOW,
            display == HotkeyDisplayMode::AltHeld,
        ),
    };
    let style = if this_held {
        Style::default()
            .fg(fg)
            .bg(held_bg)
            .add_modifier(Modifier::BOLD)
    } else {
        // Spec says "dim/light font" for the non-held family. We
        // express that as a darker BG + non-bold FG so the badge stays
        // legibly framed but visually subordinate to the held one,
        // without relying on the `Modifier::DIM` SGR.
        Style::default().fg(fg).bg(dim_bg)
    };
    Some(Span::styled(badge_text(hotkey), style))
}
