//! Locale detection from environment variables.
//!
//! This module provides functionality for detecting system locale settings
//! from standard environment variables (LANG, LC_ALL, LC_CTYPE, etc.).

use serde::{Deserialize, Serialize};

// ============================================================================
// Locale Detection
// ============================================================================

/// Locale information from environment variables.
///
/// Contains the various LC_* and LANG environment variables used to
/// configure system locale settings. Also provides extracted language
/// code and encoding from the highest-priority locale setting.
///
/// ## Locale String Format
///
/// Locale strings follow the format: `language[_territory][.codeset][@modifier]`
///
/// Examples:
/// - `en_US.UTF-8` - English, United States, UTF-8 encoding
/// - `de_DE.ISO-8859-1` - German, Germany, ISO-8859-1 encoding
/// - `zh_CN.GB18030` - Chinese, China, GB18030 encoding
/// - `C` or `POSIX` - Minimal/portable locale
///
/// ## Priority Order
///
/// The preferred language is determined by checking (in order):
/// 1. `LC_ALL` - Overrides all other LC_* variables
/// 2. `LC_MESSAGES` - Language for messages
/// 3. `LANG` - Default locale
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LocaleInfo {
    /// LANG environment variable (default locale)
    pub lang: Option<String>,
    /// LC_ALL environment variable (highest priority, overrides all others)
    pub lc_all: Option<String>,
    /// LC_CTYPE environment variable (character classification)
    pub lc_ctype: Option<String>,
    /// LC_MESSAGES environment variable (message language)
    pub lc_messages: Option<String>,
    /// LC_TIME environment variable (date/time formatting)
    pub lc_time: Option<String>,
    /// Extracted language code (e.g., "en" from "en_US.UTF-8")
    pub preferred_language: Option<String>,
    /// Extracted encoding (e.g., "UTF-8" from "en_US.UTF-8")
    pub encoding: Option<String>,
}

/// Detects locale information from environment variables.
///
/// Reads the standard locale environment variables (LANG, LC_ALL, LC_CTYPE,
/// LC_MESSAGES, LC_TIME) and extracts the preferred language and encoding
/// based on priority rules.
///
/// ## Examples
///
/// ```
/// use sniff_lib::os::detect_locale;
///
/// let locale = detect_locale();
/// if let Some(lang) = &locale.preferred_language {
///     println!("Preferred language: {}", lang);
/// }
/// if let Some(enc) = &locale.encoding {
///     println!("Encoding: {}", enc);
/// }
/// ```
///
/// ## Priority Rules
///
/// The preferred language and encoding are extracted from the first
/// non-empty, non-"C", non-"POSIX" value in this order:
/// 1. `LC_ALL`
/// 2. `LC_MESSAGES`
/// 3. `LANG`
pub fn detect_locale() -> LocaleInfo {
    let lang = std::env::var("LANG").ok();
    let lc_all = std::env::var("LC_ALL").ok();
    let lc_ctype = std::env::var("LC_CTYPE").ok();
    let lc_messages = std::env::var("LC_MESSAGES").ok();
    let lc_time = std::env::var("LC_TIME").ok();

    // Determine the effective locale for language/encoding extraction
    // Priority: LC_ALL > LC_MESSAGES > LANG
    let effective_locale = lc_all
        .as_deref()
        .filter(|s| is_extractable_locale(s))
        .or_else(|| lc_messages.as_deref().filter(|s| is_extractable_locale(s)))
        .or_else(|| lang.as_deref().filter(|s| is_extractable_locale(s)));

    let preferred_language = effective_locale.and_then(extract_language_code);
    let encoding = effective_locale.and_then(extract_encoding);

    LocaleInfo {
        lang,
        lc_all,
        lc_ctype,
        lc_messages,
        lc_time,
        preferred_language,
        encoding,
    }
}

/// Checks if a locale string can yield meaningful language/encoding info.
///
/// Returns `false` for empty strings, "C", and "POSIX" locales which
/// don't contain extractable language or encoding information.
fn is_extractable_locale(locale: &str) -> bool {
    !locale.is_empty() && locale != "C" && locale != "POSIX"
}

/// Extracts the language code from a locale string.
///
/// Parses locale strings in the format `language[_territory][.codeset][@modifier]`
/// and returns just the language portion.
///
/// ## Examples
///
/// ```
/// use sniff_lib::os::extract_language_code;
///
/// assert_eq!(extract_language_code("en_US.UTF-8"), Some("en".to_string()));
/// assert_eq!(extract_language_code("de_DE"), Some("de".to_string()));
/// assert_eq!(extract_language_code("zh"), Some("zh".to_string()));
/// assert_eq!(extract_language_code("C"), None);
/// assert_eq!(extract_language_code("POSIX"), None);
/// ```
///
/// ## Returns
///
/// - `Some(language)` - The extracted language code
/// - `None` - If the locale is empty, "C", or "POSIX"
pub fn extract_language_code(locale: &str) -> Option<String> {
    if locale.is_empty() || locale == "C" || locale == "POSIX" {
        return None;
    }

    // Extract language before any separator (_, ., @)
    let language = locale.split(['_', '.', '@']).next()?;

    if language.is_empty() {
        None
    } else {
        Some(language.to_string())
    }
}

/// Extracts the encoding from a locale string.
///
/// Parses locale strings in the format `language[_territory][.codeset][@modifier]`
/// and returns just the codeset/encoding portion.
///
/// ## Examples
///
/// ```
/// use sniff_lib::os::extract_encoding;
///
/// assert_eq!(extract_encoding("en_US.UTF-8"), Some("UTF-8".to_string()));
/// assert_eq!(extract_encoding("de_DE.ISO-8859-1"), Some("ISO-8859-1".to_string()));
/// assert_eq!(extract_encoding("zh_CN.GB18030@stroke"), Some("GB18030".to_string()));
/// assert_eq!(extract_encoding("en_US"), None);
/// assert_eq!(extract_encoding("C"), None);
/// ```
///
/// ## Returns
///
/// - `Some(encoding)` - The extracted encoding/codeset
/// - `None` - If no encoding is present in the locale string
pub fn extract_encoding(locale: &str) -> Option<String> {
    if locale.is_empty() || locale == "C" || locale == "POSIX" {
        return None;
    }

    // Find the position after the dot
    let dot_pos = locale.find('.')?;
    let after_dot = &locale[dot_pos + 1..];

    if after_dot.is_empty() {
        return None;
    }

    // Extract encoding up to any modifier (@)
    let encoding = after_dot.split('@').next()?;

    if encoding.is_empty() {
        None
    } else {
        Some(encoding.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // ========== Locale Tests ==========

    mod extract_language_code_tests {
        use super::*;

        #[test]
        fn test_full_locale_with_encoding() {
            assert_eq!(extract_language_code("en_US.UTF-8"), Some("en".to_string()));
        }

        #[test]
        fn test_locale_with_territory_only() {
            assert_eq!(extract_language_code("de_DE"), Some("de".to_string()));
        }

        #[test]
        fn test_language_only() {
            assert_eq!(extract_language_code("zh"), Some("zh".to_string()));
        }

        #[test]
        fn test_locale_with_modifier() {
            assert_eq!(extract_language_code("sr_RS@latin"), Some("sr".to_string()));
        }

        #[test]
        fn test_full_locale_with_modifier() {
            assert_eq!(
                extract_language_code("zh_CN.GB18030@stroke"),
                Some("zh".to_string())
            );
        }

        #[test]
        fn test_c_locale_returns_none() {
            assert_eq!(extract_language_code("C"), None);
        }

        #[test]
        fn test_posix_locale_returns_none() {
            assert_eq!(extract_language_code("POSIX"), None);
        }

        #[test]
        fn test_empty_string_returns_none() {
            assert_eq!(extract_language_code(""), None);
        }

        #[test]
        fn test_iso_encoding() {
            assert_eq!(
                extract_language_code("de_DE.ISO-8859-1"),
                Some("de".to_string())
            );
        }
    }

    mod extract_encoding_tests {
        use super::*;

        #[test]
        fn test_utf8_encoding() {
            assert_eq!(extract_encoding("en_US.UTF-8"), Some("UTF-8".to_string()));
        }

        #[test]
        fn test_iso_encoding() {
            assert_eq!(
                extract_encoding("de_DE.ISO-8859-1"),
                Some("ISO-8859-1".to_string())
            );
        }

        #[test]
        fn test_gb18030_encoding() {
            assert_eq!(
                extract_encoding("zh_CN.GB18030"),
                Some("GB18030".to_string())
            );
        }

        #[test]
        fn test_encoding_with_modifier() {
            assert_eq!(
                extract_encoding("zh_CN.GB18030@stroke"),
                Some("GB18030".to_string())
            );
        }

        #[test]
        fn test_no_encoding_returns_none() {
            assert_eq!(extract_encoding("en_US"), None);
        }

        #[test]
        fn test_language_only_returns_none() {
            assert_eq!(extract_encoding("en"), None);
        }

        #[test]
        fn test_c_locale_returns_none() {
            assert_eq!(extract_encoding("C"), None);
        }

        #[test]
        fn test_posix_locale_returns_none() {
            assert_eq!(extract_encoding("POSIX"), None);
        }

        #[test]
        fn test_empty_string_returns_none() {
            assert_eq!(extract_encoding(""), None);
        }

        #[test]
        fn test_trailing_dot_returns_none() {
            assert_eq!(extract_encoding("en_US."), None);
        }
    }

    mod detect_locale_tests {
        use super::*;

        // Mutex to ensure env var tests don't interfere with each other
        static ENV_MUTEX: Mutex<()> = Mutex::new(());

        /// RAII guard for temporarily setting environment variables in tests.
        struct ScopedEnv {
            vars: Vec<(String, Option<String>)>,
        }

        impl ScopedEnv {
            fn new() -> Self {
                Self { vars: Vec::new() }
            }

            fn set(&mut self, key: &str, value: &str) -> &mut Self {
                // Store original value for restoration
                let original = std::env::var(key).ok();
                self.vars.push((key.to_string(), original));
                // SAFETY: Tests are run single-threaded with ENV_MUTEX protection,
                // and we restore the original values in Drop.
                unsafe { std::env::set_var(key, value) };
                self
            }

            fn remove(&mut self, key: &str) -> &mut Self {
                let original = std::env::var(key).ok();
                self.vars.push((key.to_string(), original));
                // SAFETY: Tests are run single-threaded with ENV_MUTEX protection,
                // and we restore the original values in Drop.
                unsafe { std::env::remove_var(key) };
                self
            }
        }

        impl Drop for ScopedEnv {
            fn drop(&mut self) {
                // Restore original values in reverse order
                for (key, original) in self.vars.iter().rev() {
                    // SAFETY: Restoring original values; tests are single-threaded.
                    match original {
                        Some(value) => unsafe { std::env::set_var(key, value) },
                        None => unsafe { std::env::remove_var(key) },
                    }
                }
            }
        }

        #[test]
        fn test_detect_locale_reads_lang() {
            let _lock = ENV_MUTEX.lock().unwrap();
            let mut env = ScopedEnv::new();
            env.set("LANG", "en_US.UTF-8")
                .remove("LC_ALL")
                .remove("LC_MESSAGES");

            let locale = detect_locale();

            assert_eq!(locale.lang, Some("en_US.UTF-8".to_string()));
            assert_eq!(locale.preferred_language, Some("en".to_string()));
            assert_eq!(locale.encoding, Some("UTF-8".to_string()));
        }

        #[test]
        fn test_lc_all_takes_priority_over_lang() {
            let _lock = ENV_MUTEX.lock().unwrap();
            let mut env = ScopedEnv::new();
            env.set("LANG", "en_US.UTF-8")
                .set("LC_ALL", "de_DE.ISO-8859-1")
                .remove("LC_MESSAGES");

            let locale = detect_locale();

            assert_eq!(locale.preferred_language, Some("de".to_string()));
            assert_eq!(locale.encoding, Some("ISO-8859-1".to_string()));
        }

        #[test]
        fn test_lc_messages_takes_priority_over_lang() {
            let _lock = ENV_MUTEX.lock().unwrap();
            let mut env = ScopedEnv::new();
            env.set("LANG", "en_US.UTF-8")
                .set("LC_MESSAGES", "fr_FR.UTF-8")
                .remove("LC_ALL");

            let locale = detect_locale();

            assert_eq!(locale.preferred_language, Some("fr".to_string()));
        }

        #[test]
        fn test_lc_all_takes_priority_over_lc_messages() {
            let _lock = ENV_MUTEX.lock().unwrap();
            let mut env = ScopedEnv::new();
            env.set("LANG", "en_US.UTF-8")
                .set("LC_MESSAGES", "fr_FR.UTF-8")
                .set("LC_ALL", "ja_JP.UTF-8");

            let locale = detect_locale();

            assert_eq!(locale.preferred_language, Some("ja".to_string()));
        }

        #[test]
        fn test_c_locale_is_skipped_in_priority() {
            let _lock = ENV_MUTEX.lock().unwrap();
            let mut env = ScopedEnv::new();
            env.set("LANG", "en_US.UTF-8")
                .set("LC_ALL", "C")
                .remove("LC_MESSAGES");

            let locale = detect_locale();

            // LC_ALL is "C" so should fall back to LANG
            assert_eq!(locale.preferred_language, Some("en".to_string()));
            assert_eq!(locale.lc_all, Some("C".to_string())); // But LC_ALL is still captured
        }

        #[test]
        fn test_posix_locale_is_skipped_in_priority() {
            let _lock = ENV_MUTEX.lock().unwrap();
            let mut env = ScopedEnv::new();
            env.set("LANG", "de_DE.UTF-8")
                .set("LC_ALL", "POSIX")
                .remove("LC_MESSAGES");

            let locale = detect_locale();

            assert_eq!(locale.preferred_language, Some("de".to_string()));
        }

        #[test]
        fn test_all_lc_vars_captured() {
            let _lock = ENV_MUTEX.lock().unwrap();
            let mut env = ScopedEnv::new();
            env.set("LANG", "en_US.UTF-8")
                .set("LC_ALL", "de_DE.UTF-8")
                .set("LC_CTYPE", "fr_FR.UTF-8")
                .set("LC_MESSAGES", "ja_JP.UTF-8")
                .set("LC_TIME", "zh_CN.UTF-8");

            let locale = detect_locale();

            assert_eq!(locale.lang, Some("en_US.UTF-8".to_string()));
            assert_eq!(locale.lc_all, Some("de_DE.UTF-8".to_string()));
            assert_eq!(locale.lc_ctype, Some("fr_FR.UTF-8".to_string()));
            assert_eq!(locale.lc_messages, Some("ja_JP.UTF-8".to_string()));
            assert_eq!(locale.lc_time, Some("zh_CN.UTF-8".to_string()));
        }

        #[test]
        fn test_missing_vars_are_none() {
            let _lock = ENV_MUTEX.lock().unwrap();
            let mut env = ScopedEnv::new();
            env.remove("LANG")
                .remove("LC_ALL")
                .remove("LC_CTYPE")
                .remove("LC_MESSAGES")
                .remove("LC_TIME");

            let locale = detect_locale();

            assert!(locale.lang.is_none());
            assert!(locale.lc_all.is_none());
            assert!(locale.lc_ctype.is_none());
            assert!(locale.lc_messages.is_none());
            assert!(locale.lc_time.is_none());
            assert!(locale.preferred_language.is_none());
            assert!(locale.encoding.is_none());
        }

        #[test]
        fn test_locale_info_default() {
            let info = LocaleInfo::default();
            assert!(info.lang.is_none());
            assert!(info.lc_all.is_none());
            assert!(info.lc_ctype.is_none());
            assert!(info.lc_messages.is_none());
            assert!(info.lc_time.is_none());
            assert!(info.preferred_language.is_none());
            assert!(info.encoding.is_none());
        }

        #[test]
        fn test_locale_info_serialization() {
            let locale = LocaleInfo {
                lang: Some("en_US.UTF-8".to_string()),
                lc_all: None,
                lc_ctype: Some("en_US.UTF-8".to_string()),
                lc_messages: None,
                lc_time: None,
                preferred_language: Some("en".to_string()),
                encoding: Some("UTF-8".to_string()),
            };

            let json = serde_json::to_string(&locale).unwrap();
            let deserialized: LocaleInfo = serde_json::from_str(&json).unwrap();

            assert_eq!(locale.lang, deserialized.lang);
            assert_eq!(locale.preferred_language, deserialized.preferred_language);
            assert_eq!(locale.encoding, deserialized.encoding);
        }
    }
}
