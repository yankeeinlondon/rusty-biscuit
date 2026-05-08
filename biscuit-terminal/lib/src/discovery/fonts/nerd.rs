//! Nerd Font detection helpers.

/// Known Nerd Font family base names (without "Nerd Font" suffix).
///
/// This list is sourced from the [Nerd Fonts GitHub repository](https://github.com/ryanoasis/nerd-fonts).
/// Last updated: 2026-01.
const NERD_FONT_BASE_NAMES: &[&str] = &[
    "0xProto",
    "3270",
    "AdwaitaMono",
    "Agave",
    "AnonymicePro",
    "Arimo",
    "AtkynsonMono",
    "AurulentSansMono",
    "BigBlueTerminal",
    "BitstromWera",
    "BlexMono",
    "CaskaydiaCove",
    "CaskaydiaMono",
    "CodeNewRoman",
    "ComicShannsMono",
    "CommitMono",
    "Cousine",
    "D2Coding",
    "DaddyTimeMono",
    "DepartureMono",
    "DejaVuSansMono",
    "DroidSansMono",
    "EnvyCodeR",
    "FantasqueSansMono",
    "FiraCode",
    "FiraMono",
    "GeistMono",
    "GoMono",
    "Gohu",
    "Hack",
    "Hasklug",
    "HeavyDataMono",
    "Hurmit",
    "iM-Writing",
    "Inconsolata",
    "InconsolataGo",
    "InconsolataLGC",
    "IntoneMono",
    "Iosevka",
    "IosevkaTerm",
    "IosevkaTermSlab",
    "JetBrainsMono",
    "Lekton",
    "Literation",
    "Lilex",
    "MartianMono",
    "Meslo",
    "Monaspice",
    "Monofur",
    "Monoid",
    "Mononoki",
    "MPlus",
    "Noto",
    "OpenDyslexic",
    "Overpass",
    "ProFont",
    "ProggyClean",
    "RecMono",
    "RobotoMono",
    "SauceCodePro",
    "ShureTechMono",
    "SpaceMono",
    "Terminess",
    "Tinos",
    "Ubuntu",
    "UbuntuMono",
    "UbuntuSans",
    "VictorMono",
    "ZedMono",
];

/// Check if a font name matches a known Nerd Font.
///
/// Detection uses three strategies:
/// 1. **Suffix marker**: Names containing "Nerd Font" or ending with " NF"
/// 2. **Base name match**: Names matching known Nerd Font base names (normalized)
/// 3. **Prefix match**: Font names starting with a known base name
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::fonts::is_nerd_font_name;
///
/// assert!(is_nerd_font_name("JetBrainsMono Nerd Font"));
/// assert!(is_nerd_font_name("FiraCode NF"));
/// assert!(is_nerd_font_name("Hack"));
/// assert!(!is_nerd_font_name("Monaco"));
/// ```
pub fn is_nerd_font_name(font_name: &str) -> bool {
    let lower = font_name.to_lowercase();
    // Normalize: remove spaces for comparison (JetBrains Mono -> jetbrainsmono)
    let normalized = lower.replace([' ', '-'], "");

    // 1. Check for explicit "Nerd Font" or "NF" markers (definite match)
    if lower.contains("nerd font") || lower.ends_with(" nf") || lower.contains(" nf ") {
        return true;
    }

    // 2. Check if the font name matches a known Nerd Font base name
    // These are fonts that have Nerd Font patched versions available
    for base in NERD_FONT_BASE_NAMES {
        let base_lower = base.to_lowercase();
        let base_normalized = base_lower.replace([' ', '-'], "");

        // Exact match (normalized)
        if normalized == base_normalized {
            return true;
        }

        // Font name starts with the base name (e.g., "JetBrainsMono Regular")
        if normalized.starts_with(&base_normalized) {
            return true;
        }

        // Base name with spaces (e.g., "JetBrains Mono" matches "JetBrainsMono")
        if lower.starts_with(&base_lower) {
            return true;
        }
    }

    false
}

/// Detect if a Nerd Font is being used.
///
/// Detection strategy:
/// 1. Check `NERD_FONT` env var (explicit user declaration)
/// 2. Check detected font name against known patterns
///
/// ## Returns
///
/// - `Some(true)`: Nerd Font confirmed (env var or font name match)
/// - `Some(false)`: Explicitly disabled via env var
/// - `None`: Cannot determine (no env var, unknown font)
///
/// ## Environment Variable
///
/// The `NERD_FONT` environment variable is a community convention:
/// - `NERD_FONT=1` or `NERD_FONT=true` - Explicitly declare Nerd Font usage
/// - `NERD_FONT=0` or `NERD_FONT=false` - Explicitly declare no Nerd Font
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::fonts::detect_nerd_font;
///
/// // Returns Some(true), Some(false), or None based on environment
/// let nerd_font = detect_nerd_font();
/// if nerd_font == Some(true) {
///     println!("Nerd Font icons available!");
/// }
/// ```
pub fn detect_nerd_font() -> Option<bool> {
    // Check NERD_FONT environment variable first (explicit user declaration)
    if let Ok(value) = std::env::var("NERD_FONT") {
        let lower = value.to_lowercase();
        if lower == "1" || lower == "true" || lower == "yes" {
            tracing::debug!("detect_nerd_font(): NERD_FONT env var is set to true");
            return Some(true);
        }
        if lower == "0" || lower == "false" || lower == "no" {
            tracing::debug!("detect_nerd_font(): NERD_FONT env var is set to false");
            return Some(false);
        }
        // Non-standard value, ignore
        tracing::debug!(
            "detect_nerd_font(): NERD_FONT env var has non-standard value: {}",
            value
        );
    }

    // Check detected font name
    if let Some(name) = super::font_name() {
        if is_nerd_font_name(&name) {
            tracing::debug!("detect_nerd_font(): font '{}' detected as Nerd Font", name);
            return Some(true);
        }
        tracing::debug!(
            "detect_nerd_font(): font '{}' is not a known Nerd Font",
            name
        );
    }

    // Cannot determine
    tracing::debug!("detect_nerd_font(): cannot determine Nerd Font status");
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_nerd_font_name_with_nerd_font_suffix() {
        assert!(is_nerd_font_name("JetBrainsMono Nerd Font"));
        assert!(is_nerd_font_name("Hack Nerd Font Mono"));
        assert!(is_nerd_font_name("Fira Code Nerd Font"));
        assert!(is_nerd_font_name("Meslo LG S Nerd Font"));
    }

    #[test]
    fn test_is_nerd_font_name_with_nf_suffix() {
        assert!(is_nerd_font_name("FiraCode NF"));
        assert!(is_nerd_font_name("Meslo LG M NF"));
        assert!(is_nerd_font_name("Hack NF"));
        assert!(is_nerd_font_name("JetBrainsMono NF Mono"));
    }

    #[test]
    fn test_is_nerd_font_name_case_insensitive() {
        assert!(is_nerd_font_name("jetbrainsmono nerd font"));
        assert!(is_nerd_font_name("HACK NF"));
        assert!(is_nerd_font_name("FiraCode NERD FONT"));
    }

    #[test]
    fn test_is_nerd_font_name_non_nerd_fonts() {
        assert!(!is_nerd_font_name("Monaco"));
        assert!(!is_nerd_font_name("SF Mono"));
        assert!(!is_nerd_font_name("Menlo"));
        assert!(!is_nerd_font_name("Courier New"));
        assert!(!is_nerd_font_name("Consolas"));
        assert!(!is_nerd_font_name("Arial"));
        assert!(!is_nerd_font_name("Helvetica"));
    }

    #[test]
    fn test_is_nerd_font_name_base_names_recognized() {
        assert!(is_nerd_font_name("JetBrains Mono"));
        assert!(is_nerd_font_name("JetBrainsMono"));
        assert!(is_nerd_font_name("Fira Code"));
        assert!(is_nerd_font_name("FiraCode"));
        assert!(is_nerd_font_name("Hack"));
        assert!(is_nerd_font_name("Meslo"));
        assert!(is_nerd_font_name("Iosevka"));
        assert!(is_nerd_font_name("Victor Mono"));
    }

    #[test]
    fn test_is_nerd_font_name_with_style_suffix() {
        assert!(is_nerd_font_name("JetBrainsMono Regular"));
        assert!(is_nerd_font_name("Hack Bold"));
        assert!(is_nerd_font_name("FiraCode Light"));
    }

    #[test]
    #[serial_test::serial]
    fn test_detect_nerd_font_env_var_true() {
        unsafe { std::env::set_var("NERD_FONT", "1") };
        let result = detect_nerd_font();
        unsafe { std::env::remove_var("NERD_FONT") };
        assert_eq!(result, Some(true));
    }

    #[test]
    #[serial_test::serial]
    fn test_detect_nerd_font_env_var_true_word() {
        unsafe { std::env::set_var("NERD_FONT", "true") };
        let result = detect_nerd_font();
        unsafe { std::env::remove_var("NERD_FONT") };
        assert_eq!(result, Some(true));
    }

    #[test]
    #[serial_test::serial]
    fn test_detect_nerd_font_env_var_false() {
        unsafe { std::env::set_var("NERD_FONT", "0") };
        let result = detect_nerd_font();
        unsafe { std::env::remove_var("NERD_FONT") };
        assert_eq!(result, Some(false));
    }

    #[test]
    #[serial_test::serial]
    fn test_detect_nerd_font_env_var_false_word() {
        unsafe { std::env::set_var("NERD_FONT", "false") };
        let result = detect_nerd_font();
        unsafe { std::env::remove_var("NERD_FONT") };
        assert_eq!(result, Some(false));
    }

    #[test]
    fn test_detect_nerd_font_does_not_panic() {
        let _ = detect_nerd_font();
    }
}
