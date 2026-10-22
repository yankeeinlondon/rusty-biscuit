//! Font files compiled into the library, so every consumer binary has them.
//!
//! - **Inter** (Regular and Bold, SIL OFL 1.1) is registered in the font
//!   database as the family `Inter`, matched only by name (`MERMAID_FONT` or
//!   a CSS font stack). It is not an installed face, so it is never a
//!   candidate for the installed sans-serif step of font resolution.
//! - **Noto Emoji** (monochrome, SIL OFL 1.1) exists only in Linux builds and
//!   is the last fallback face, never matched by name.
//!
//! Sources, licenses, and checksums: `assets/fonts/*/README.md`.

/// Inter Regular (weight 400, upright), static TrueType.
pub(crate) static INTER_REGULAR: &[u8] =
    include_bytes!("../../assets/fonts/inter/Inter-Regular.ttf");

/// Inter Bold (weight 700, upright), static TrueType.
pub(crate) static INTER_BOLD: &[u8] = include_bytes!("../../assets/fonts/inter/Inter-Bold.ttf");

/// Noto Emoji, a variable TrueType font (`wght` 300–700) whose default
/// instance is Regular 400.
#[cfg(target_os = "linux")]
pub(crate) static NOTO_EMOJI: &[u8] =
    include_bytes!("../../assets/fonts/noto-emoji/NotoEmoji-Variable.ttf");
