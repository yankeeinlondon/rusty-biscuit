//! Normalization of raw option strings into [`ChoiceOption`] values.
//!
//! Handles hotkey prefix parsing, label/value convention transforms,
//! `::` delimiter splitting, and numeric hotkey assignment.

use tui_chrome::{ChoiceOption, HotkeySpec};

/// Errors that can occur during option normalization.
#[derive(Debug, thiserror::Error)]
pub enum NormalizeError {
    #[error("duplicate hotkey '{hotkey}' on options: '{first}' and '{second}'")]
    DuplicateHotkey {
        hotkey: String,
        first: String,
        second: String,
    },
}

/// A convention for transforming strings into labels or values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum NamingConvention {
    #[default]
    None,
    CamelCase,
    PascalCase,
    KebabCase,
    SnakeCase,
    TitleCase,
    Caps,
    Lowercase,
}

/// Parsed representation of a raw option string before normalization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedOption {
    pub raw: String,
    pub label: String,
    pub value: String,
    pub hotkey: Option<HotkeySpec>,
}

/// Parses a raw option string, extracting any hotkey prefix and splitting
/// on `::` if present.
///
/// Hotkey prefixes:
/// - `[CTRL+X]` → `HotkeySpec::Ctrl('x')`
/// - `[ALT+X]` or `[OPT+X]` → `HotkeySpec::Alt('x')`
pub fn parse_option(raw: &str) -> ParsedOption {
    let (hotkey, rest) = extract_hotkey(raw);
    let rest = rest.trim();

    // Split on :: delimiter
    if let Some((label, value)) = rest.split_once("::") {
        let label = label.trim();
        let value = value.trim();
        ParsedOption {
            raw: raw.to_string(),
            label: label.to_string(),
            value: value.to_string(),
            hotkey,
        }
    } else {
        ParsedOption {
            raw: raw.to_string(),
            label: rest.to_string(),
            value: rest.to_string(),
            hotkey,
        }
    }
}

fn extract_hotkey(s: &str) -> (Option<HotkeySpec>, &str) {
    let trimmed = s.trim_start();
    if let Some(rest) = trimmed.strip_prefix('[')
        && let Some(end_idx) = rest.find(']')
    {
        let spec = &rest[..end_idx];
        let after = &rest[end_idx + 1..];
        if let Some(hotkey) = parse_hotkey_spec(spec) {
            return (Some(hotkey), after);
        }
    }
    (None, s)
}

fn parse_hotkey_spec(spec: &str) -> Option<HotkeySpec> {
    let spec = spec.trim();
    if spec.to_uppercase().starts_with("CTRL+") {
        let ch = spec.chars().nth(5)?;
        return Some(HotkeySpec::Ctrl(ch.to_ascii_lowercase()));
    }
    if spec.to_uppercase().starts_with("ALT+") {
        let ch = spec.chars().nth(4)?;
        return Some(HotkeySpec::Alt(ch.to_ascii_lowercase()));
    }
    if spec.to_uppercase().starts_with("OPT+") {
        let ch = spec.chars().nth(4)?;
        return Some(HotkeySpec::Alt(ch.to_ascii_lowercase()));
    }
    None
}

/// Applies a naming convention to a string.
pub fn apply_convention(s: &str, convention: NamingConvention) -> String {
    match convention {
        NamingConvention::None => s.to_string(),
        NamingConvention::CamelCase => to_camel_case(s),
        NamingConvention::PascalCase => to_pascal_case(s),
        NamingConvention::KebabCase => to_kebab_case(s),
        NamingConvention::SnakeCase => to_snake_case(s),
        NamingConvention::TitleCase => to_title_case(s),
        NamingConvention::Caps => s.to_uppercase(),
        NamingConvention::Lowercase => s.to_lowercase(),
    }
}

fn to_camel_case(s: &str) -> String {
    let words: Vec<&str> = s.split_whitespace().collect();
    if words.is_empty() {
        return String::new();
    }
    let mut result = words[0].to_lowercase();
    for word in &words[1..] {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            result.push(first.to_uppercase().next().unwrap_or(first));
            result.extend(chars.flat_map(|c| c.to_lowercase()));
        }
    }
    result
}

fn to_pascal_case(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => {
                    let mut result = first.to_uppercase().collect::<String>();
                    result.extend(chars.flat_map(|c| c.to_lowercase()));
                    result
                }
                None => String::new(),
            }
        })
        .collect()
}

fn to_kebab_case(s: &str) -> String {
    s.split_whitespace()
        .map(|w| w.to_lowercase())
        .collect::<Vec<_>>()
        .join("-")
}

fn to_snake_case(s: &str) -> String {
    s.split_whitespace()
        .map(|w| w.to_lowercase())
        .collect::<Vec<_>>()
        .join("_")
}

fn to_title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => {
                    let mut result = first.to_uppercase().collect::<String>();
                    result.extend(chars.flat_map(|c| c.to_lowercase()));
                    result
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Assigns numeric hotkeys to options when enabled.
///
/// First 10 options get Ctrl+1 through Ctrl+9, then Ctrl+0.
/// Next 10 options get Alt+1 through Alt+9, then Alt+0.
/// Explicit hotkeys are never overwritten.
pub fn assign_numeric_hotkeys(options: &mut [ParsedOption]) {
    for (idx, option) in options.iter_mut().enumerate() {
        if option.hotkey.is_some() {
            continue;
        }
        if idx < 10 {
            let ch = if idx == 9 {
                '0'
            } else {
                (b'1' + idx as u8) as char
            };
            option.hotkey = Some(HotkeySpec::Ctrl(ch));
        } else if idx < 20 {
            let ch = if idx == 19 {
                '0'
            } else {
                (b'1' + (idx - 10) as u8) as char
            };
            option.hotkey = Some(HotkeySpec::Alt(ch));
        }
    }
}

/// Normalizes a list of raw option strings into [`ChoiceOption`] values.
///
/// ## Arguments
///
/// * `raw_options` - The raw strings from source resolution.
/// * `label_convention` - Optional convention to apply to labels.
/// * `value_convention` - Optional convention to apply to values.
/// * `numeric_hotkeys` - Whether to assign numeric hotkeys.
/// * `delimiter` - Legacy delimiter for splitting (ignored if `::` is present).
///
/// ## Errors
///
/// Returns an error if duplicate hotkeys are detected.
pub fn normalize_options(
    raw_options: Vec<String>,
    label_convention: NamingConvention,
    value_convention: NamingConvention,
    numeric_hotkeys: bool,
    delimiter: Option<char>,
) -> Result<Vec<ChoiceOption<String>>, NormalizeError> {
    let mut parsed: Vec<ParsedOption> = raw_options
        .into_iter()
        .map(|raw| {
            if raw.contains("::") {
                parse_option(&raw)
            } else if let Some(delim) = delimiter {
                if let Some((label, value)) = raw.split_once(delim) {
                    ParsedOption {
                        raw: raw.clone(),
                        label: label.trim().to_string(),
                        value: value.trim().to_string(),
                        hotkey: None,
                    }
                } else {
                    parse_option(&raw)
                }
            } else {
                parse_option(&raw)
            }
        })
        .collect();

    if numeric_hotkeys {
        assign_numeric_hotkeys(&mut parsed);
    }

    // Check for duplicate hotkeys
    let mut seen: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for option in &parsed {
        if let Some(hotkey) = option.hotkey {
            let key = format!("{:?}", hotkey);
            if let Some(first) = seen.get(&key) {
                return Err(NormalizeError::DuplicateHotkey {
                    hotkey: key,
                    first: first.clone(),
                    second: option.raw.clone(),
                });
            }
            seen.insert(key, option.raw.clone());
        }
    }

    Ok(parsed
        .into_iter()
        .map(|opt| {
            let id = apply_convention(&opt.value, value_convention);
            let label = apply_convention(&opt.label, label_convention);
            let value = apply_convention(&opt.value, value_convention);
            ChoiceOption::new(id, label, value).with_hotkey_option(opt.hotkey)
        })
        .collect())
}

trait WithHotkeyOption {
    fn with_hotkey_option(self, hotkey: Option<HotkeySpec>) -> Self;
}

impl WithHotkeyOption for ChoiceOption<String> {
    fn with_hotkey_option(self, hotkey: Option<HotkeySpec>) -> Self {
        match hotkey {
            Some(h) => self.with_hotkey(h),
            None => self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_option_without_hotkey_or_delimiter() {
        let parsed = parse_option("Apple");
        assert_eq!(parsed.label, "Apple");
        assert_eq!(parsed.value, "Apple");
        assert!(parsed.hotkey.is_none());
    }

    #[test]
    fn parse_option_with_delimiter() {
        let parsed = parse_option("Red::apple");
        assert_eq!(parsed.label, "Red");
        assert_eq!(parsed.value, "apple");
    }

    #[test]
    fn parse_option_with_ctrl_hotkey() {
        let parsed = parse_option("[CTRL+r] Red");
        assert_eq!(parsed.hotkey, Some(HotkeySpec::Ctrl('r')));
        assert_eq!(parsed.label, "Red");
    }

    #[test]
    fn parse_option_with_alt_hotkey() {
        let parsed = parse_option("[ALT+a] Apple");
        assert_eq!(parsed.hotkey, Some(HotkeySpec::Alt('a')));
        assert_eq!(parsed.label, "Apple");
    }

    #[test]
    fn parse_option_with_opt_alias() {
        let parsed = parse_option("[OPT+o] Orange");
        assert_eq!(parsed.hotkey, Some(HotkeySpec::Alt('o')));
    }

    #[test]
    fn hotkey_normalization_lowercases() {
        let parsed = parse_option("[CTRL+R] Red");
        assert_eq!(parsed.hotkey, Some(HotkeySpec::Ctrl('r')));
    }

    #[test]
    fn apply_convention_kebab_case() {
        assert_eq!(
            apply_convention("Hello World", NamingConvention::KebabCase),
            "hello-world"
        );
    }

    #[test]
    fn apply_convention_snake_case() {
        assert_eq!(
            apply_convention("Hello World", NamingConvention::SnakeCase),
            "hello_world"
        );
    }

    #[test]
    fn apply_convention_camel_case() {
        assert_eq!(
            apply_convention("hello world", NamingConvention::CamelCase),
            "helloWorld"
        );
    }

    #[test]
    fn apply_convention_pascal_case() {
        assert_eq!(
            apply_convention("hello world", NamingConvention::PascalCase),
            "HelloWorld"
        );
    }

    #[test]
    fn apply_convention_title_case() {
        assert_eq!(
            apply_convention("hello world", NamingConvention::TitleCase),
            "Hello World"
        );
    }

    #[test]
    fn apply_convention_caps() {
        assert_eq!(apply_convention("hello", NamingConvention::Caps), "HELLO");
    }

    #[test]
    fn apply_convention_lowercase() {
        assert_eq!(
            apply_convention("HELLO", NamingConvention::Lowercase),
            "hello"
        );
    }

    #[test]
    fn numeric_hotkeys_first_ten_ctrl() {
        let mut options = vec![
            ParsedOption {
                raw: "a".into(),
                label: "a".into(),
                value: "a".into(),
                hotkey: None,
            },
            ParsedOption {
                raw: "b".into(),
                label: "b".into(),
                value: "b".into(),
                hotkey: None,
            },
        ];
        assign_numeric_hotkeys(&mut options);
        assert_eq!(options[0].hotkey, Some(HotkeySpec::Ctrl('1')));
        assert_eq!(options[1].hotkey, Some(HotkeySpec::Ctrl('2')));
    }

    #[test]
    fn numeric_hotkeys_tenth_is_ctrl_zero() {
        let mut options = vec![ParsedOption {
            raw: "a".into(),
            label: "a".into(),
            value: "a".into(),
            hotkey: None,
        }];
        assign_numeric_hotkeys(&mut options);
        assert_eq!(options[0].hotkey, Some(HotkeySpec::Ctrl('1')));
    }

    #[test]
    fn numeric_hotkeys_eleventh_is_alt_one() {
        let mut options: Vec<ParsedOption> = (0..11)
            .map(|i| ParsedOption {
                raw: format!("{}", i),
                label: format!("{}", i),
                value: format!("{}", i),
                hotkey: None,
            })
            .collect();
        assign_numeric_hotkeys(&mut options);
        assert_eq!(options[10].hotkey, Some(HotkeySpec::Alt('1')));
    }

    #[test]
    fn numeric_hotkeys_does_not_override_explicit() {
        let mut options = vec![ParsedOption {
            raw: "a".into(),
            label: "a".into(),
            value: "a".into(),
            hotkey: Some(HotkeySpec::Ctrl('x')),
        }];
        assign_numeric_hotkeys(&mut options);
        assert_eq!(options[0].hotkey, Some(HotkeySpec::Ctrl('x')));
    }

    #[test]
    fn normalize_options_basic() {
        let options = vec!["Apple".into(), "Berry".into()];
        let result = normalize_options(
            options,
            NamingConvention::None,
            NamingConvention::None,
            false,
            None,
        )
        .unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].label, "Apple");
        assert_eq!(result[0].value, "Apple");
    }

    #[test]
    fn normalize_options_with_conventions() {
        let options = vec!["hello world".into()];
        let result = normalize_options(
            options,
            NamingConvention::TitleCase,
            NamingConvention::KebabCase,
            false,
            None,
        )
        .unwrap();
        assert_eq!(result[0].label, "Hello World");
        assert_eq!(result[0].value, "hello-world");
    }

    #[test]
    fn normalize_options_detects_duplicate_hotkeys() {
        let options = vec!["[CTRL+x] A".into(), "[CTRL+x] B".into()];
        let result = normalize_options(
            options,
            NamingConvention::None,
            NamingConvention::None,
            false,
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn normalize_options_with_legacy_delimiter() {
        let options = vec!["Apple:1".into(), "Berry:2".into()];
        let result = normalize_options(
            options,
            NamingConvention::None,
            NamingConvention::None,
            false,
            Some(':'),
        )
        .unwrap();
        assert_eq!(result[0].label, "Apple");
        assert_eq!(result[0].value, "1");
        assert_eq!(result[1].label, "Berry");
        assert_eq!(result[1].value, "2");
    }

    #[test]
    fn normalize_options_explicit_delimiter_takes_precedence() {
        let options = vec!["Red::apple".into()];
        let result = normalize_options(
            options,
            NamingConvention::None,
            NamingConvention::None,
            false,
            Some(':'),
        )
        .unwrap();
        assert_eq!(result[0].label, "Red");
        assert_eq!(result[0].value, "apple");
    }
}
