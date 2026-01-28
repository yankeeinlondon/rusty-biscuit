use std::env;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use unic_langid::LanguageIdentifier;

#[derive(Debug, Clone, Copy)]
pub enum LocaleCategory {
    Messages,
    Ctype,
    Time,
    Numeric,
}

impl LocaleCategory {
    fn env_key(self) -> &'static str {
        match self {
            LocaleCategory::Messages => "LC_MESSAGES",
            LocaleCategory::Ctype => "LC_CTYPE",
            LocaleCategory::Time => "LC_TIME",
            LocaleCategory::Numeric => "LC_NUMERIC",
        }
    }
}

fn get_env_nonempty(key: &str) -> Option<String> {
    env::var(key).ok().and_then(|v| {
        let v = v.trim().to_string();
        if v.is_empty() { None } else { Some(v) }
    })
}

/// libc locale string (e.g. "en_US.UTF-8", "sr_RS@latin", "C.UTF-8") -> normalized BCP47-ish tag.
/// Returns "und" for C/POSIX-ish locales, and None if it can't form a plausible tag.
pub fn normalize_locale_to_tag(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }

    // Handle "C", "POSIX", and friends explicitly.
    let up = raw.to_ascii_uppercase();
    if up == "C" || up == "POSIX" || up == "C.UTF-8" || up == "C.UTF8" {
        return Some("und".to_string());
    }

    // Split off encoding (.UTF-8) and modifier (@latin)
    let (before_at, modifier) = raw.split_once('@').map(|(a, m)| (a, Some(m))).unwrap_or((raw, None));
    let base = before_at.split_once('.').map(|(b, _enc)| b).unwrap_or(before_at);

    // Convert to BCP47-ish: underscore -> hyphen.
    let mut tag = base.replace('_', "-");

    // Some locales are just "UTF-8" (no language). Bail.
    let up_tag = tag.to_ascii_uppercase();
    if up_tag == "UTF-8" || up_tag == "UTF8" {
        return None;
    }

    // Optionally map common glibc modifiers to script subtags.
    // Examples seen in the wild: sr_RS@latin, uz_UZ@cyrillic
    if let Some(m) = modifier {
        match m.to_ascii_lowercase().as_str() {
            "latin" => tag.push_str("-Latn"),
            "cyrillic" => tag.push_str("-Cyrl"),
            _ => {
                // Many modifiers (euro, phonebk, etc.) don't map cleanly to BCP47 without deeper rules.
                // Ignore by default to avoid lying.
            }
        }
    }

    // Now parse + re-serialize to get canonical casing/ordering.
    // LanguageIdentifier expects something close to BCP47: "en-US", "sr-Latn-RS", etc.
    let lid = LanguageIdentifier::from_str(&tag).ok()?;
    Some(lid.to_string())
}

/// Effective locale for a category (LC_ALL > LC_<CAT> > LANG) normalized to a tag.
pub fn effective_locale_tag(category: LocaleCategory) -> Option<String> {
    let raw = get_env_nonempty("LC_ALL")
        .or_else(|| get_env_nonempty(category.env_key()))
        .or_else(|| get_env_nonempty("LANG"))?;

    normalize_locale_to_tag(&raw)
}

#[cfg(windows)]
fn windows_locale_tag() -> Option<String> {
    use windows_sys::Win32::Globalization::GetUserDefaultLocaleName;

    // Per docs, max locale name length is 85 incl. null.
    let mut buf = [0u16; 85];
    let len = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32) };
    if len == 0 {
        return None;
    }
    let s = String::from_utf16_lossy(&buf[..(len as usize - 1)]);
    Some(s) // already like "en-US"
}



/// Effective locale for a specific category, following common precedence:
/// LC_ALL > LC_<CATEGORY> > LANG > "C"
pub fn effective_locale_for(category: LocaleCategory) -> String {
    get_env_nonempty("LC_ALL")
        .or_else(|| get_env_nonempty(category.env_key()))
        .or_else(|| get_env_nonempty("LANG"))
        .unwrap_or_else(|| "C".to_string())
}

/// Single “best guess” effective locale if you just want one:
/// LC_ALL > LANG > "C"
pub fn effective_locale_any() -> String {
    get_env_nonempty("LC_ALL")
        .or_else(|| get_env_nonempty("LANG"))
        .unwrap_or_else(|| "C".to_string())
}

pub fn env_says_utf8() -> Option<bool> {
    let v = get_env_nonempty("LC_ALL")
        .or_else(|| get_env_nonempty("LC_CTYPE"))
        .or_else(|| get_env_nonempty("LANG"))?;

    let upper = v.to_ascii_uppercase();
    Some(upper.contains("UTF-8") || upper.contains("UTF8"))
}



/// The Character Encoding the Terminal is using.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CharEncoding {
    /// UTF-8 (8-bit Unicode Transformation Format)
    Utf8,
    /// ASCII (American Standard Code for Information Interchange)
    Ascii,
    /// UTF-16 (16-bit Unicode Transformation Format)
    Utf16,
    /// UTF-32 (32-bit Unicode Transformation Format)
    Utf32,
    /// ISO-8859-1 (Latin-1)
    Latin1,
    /// Windows-1252 (Western European)
    Windows1252,
    /// Unknown character encoding
    Unknown,
}

impl Default for CharEncoding {
    fn default() -> Self {
        detect_char_encoding()
    }
}

/// Detect the character encoding from environment variables.
///
/// Checks `LC_CTYPE`, `LC_ALL`, and `LANG` for encoding suffixes.
/// Defaults to UTF-8 on modern systems.
pub fn detect_char_encoding() -> CharEncoding {
    let locale = get_env_nonempty("LC_ALL")
        .or_else(|| get_env_nonempty("LC_CTYPE"))
        .or_else(|| get_env_nonempty("LANG"))
        .unwrap_or_default()
        .to_ascii_uppercase();

    if locale.contains("UTF-8") || locale.contains("UTF8") {
        CharEncoding::Utf8
    } else if locale.contains("UTF-16") || locale.contains("UTF16") {
        CharEncoding::Utf16
    } else if locale.contains("UTF-32") || locale.contains("UTF32") {
        CharEncoding::Utf32
    } else if locale.contains("ISO-8859-1") || locale.contains("ISO8859-1") || locale.contains("LATIN1") {
        CharEncoding::Latin1
    } else if locale.contains("CP1252") || locale.contains("WINDOWS-1252") {
        CharEncoding::Windows1252
    } else if locale.is_empty() || locale == "C" || locale == "POSIX" {
        // Modern default - most terminals use UTF-8
        CharEncoding::Utf8
    } else {
        CharEncoding::Unknown
    }
}

/// Terminal locale information.
///
/// Contains both the raw locale string from environment variables
/// and the normalized BCP47-style locale tag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalLocale {
    /// The raw locale string from environment (e.g., "en_US.UTF-8", "C", "POSIX")
    pub raw: Option<String>,
    /// The normalized BCP47-style tag (e.g., "en-US", "und" for C/POSIX)
    pub tag: Option<String>,
}

impl Default for TerminalLocale {
    fn default() -> Self {
        let raw = get_env_nonempty("LC_ALL")
            .or_else(|| get_env_nonempty(LocaleCategory::Messages.env_key()))
            .or_else(|| get_env_nonempty("LANG"));

        let tag = raw.as_ref().and_then(|r| normalize_locale_to_tag(r));

        TerminalLocale { raw, tag }
    }
}

impl TerminalLocale {
    /// Get the normalized BCP47 locale tag, if available.
    pub fn tag(&self) -> Option<&str> {
        self.tag.as_deref()
    }

    /// Get the raw locale string from the environment.
    pub fn raw(&self) -> Option<&str> {
        self.raw.as_deref()
    }

    /// Check if a locale was detected.
    pub fn is_detected(&self) -> bool {
        self.raw.is_some()
    }

    /// Check if the locale is "C" or "POSIX" (minimal/undetermined locale).
    pub fn is_c_locale(&self) -> bool {
        self.raw
            .as_ref()
            .map(|r| {
                let up = r.to_ascii_uppercase();
                up == "C" || up == "POSIX" || up.starts_with("C.") || up.starts_with("POSIX.")
            })
            .unwrap_or(false)
    }
}
