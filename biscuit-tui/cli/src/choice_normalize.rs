//! Normalization of typed [`RawOption`] records into [`ChoiceOption`] values.
//!
//! Handles hotkey prefix parsing, label/value convention transforms,
//! `::` delimiter splitting, and numeric hotkey assignment. Object
//! sources (JSON/YAML/TOML/CSV/markdown frontmatter object arrays)
//! flow through with their `value` and `hotkey` preserved; string
//! sources fall back to the legacy prefix/delimiter parsing.

use tui_chrome::{ChoiceOption, HotkeySpec};

use crate::option_sources::RawOption;

/// Errors that can occur during option normalization.
#[derive(Debug, thiserror::Error)]
pub enum NormalizeError {
    #[error("duplicate hotkey '{hotkey}' on options: '{first}' and '{second}'")]
    DuplicateHotkey {
        hotkey: String,
        first: String,
        second: String,
    },
    #[error("invalid hotkey '{spec}' on option '{option}'")]
    InvalidHotkey { spec: String, option: String },
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
    pub disabled: bool,
}

/// Parses a raw option string, extracting any hotkey prefix and splitting
/// on `::` if present.
///
/// ## Examples
///
/// Hotkey prefixes:
/// - `[CTRL+X]` → `HotkeySpec::Ctrl('x')`
/// - `[ALT+X]` or `[OPT+X]` → `HotkeySpec::Alt('x')`
#[allow(dead_code)]
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
            disabled: false,
        }
    } else {
        ParsedOption {
            raw: raw.to_string(),
            label: rest.to_string(),
            value: rest.to_string(),
            hotkey,
            disabled: false,
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

/// Parses a canonical hotkey string such as `"CTRL+R"`, `"ALT+B"`, or
/// `"OPT+B"` (case-insensitive) into a [`HotkeySpec`].
///
/// Returns `None` when the input does not match a supported modifier
/// prefix. Object sources that supply a `hotkey` field route through
/// this same parser so the wire format matches the bracketed prefix
/// form (`[CTRL+R]`).
pub fn parse_hotkey_spec(spec: &str) -> Option<HotkeySpec> {
    let spec = spec.trim();
    let upper = spec.to_uppercase();
    if let Some(rest) = upper.strip_prefix("CTRL+") {
        let ch = rest.chars().next()?;
        return Some(HotkeySpec::Ctrl(ch.to_ascii_lowercase()));
    }
    if let Some(rest) = upper.strip_prefix("ALT+") {
        let ch = rest.chars().next()?;
        return Some(HotkeySpec::Alt(ch.to_ascii_lowercase()));
    }
    if let Some(rest) = upper.strip_prefix("OPT+") {
        let ch = rest.chars().next()?;
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

/// Normalizes a list of typed [`RawOption`] records into
/// [`ChoiceOption`] values.
///
/// ## Pipeline
///
/// 1. For each `RawOption`, start from `label` / `value` / `hotkey` /
///    `disabled` as supplied.
/// 2. When `hotkey` is `None`, strip a `[CTRL+X]` / `[ALT+X]` /
///    `[OPT+X]` prefix from the label and use it as the hotkey.
///    Object-supplied hotkey strings parse via [`parse_hotkey_spec`].
/// 3. When `value` is `None`, split the (post-prefix) label on the
///    first `::` to produce `(label, value)`. The legacy `--delimiter`
///    char is consulted as a fallback when `::` is not present.
/// 4. Apply `--label-convention` and `--value-convention` transforms
///    to the resulting label and value.
/// 5. When `--numeric-hot-keys` is set, fill in any remaining
///    `hotkey == None` slots with `Ctrl+1..0` then `Alt+1..0`.
///
/// ## Errors
///
/// - [`NormalizeError::DuplicateHotkey`] when two options share the
///   same hotkey.
/// - [`NormalizeError::InvalidHotkey`] when an object source supplies
///   a `hotkey` string that does not parse via
///   [`parse_hotkey_spec`].
pub fn normalize_options(
    raw_options: Vec<RawOption>,
    label_convention: NamingConvention,
    value_convention: NamingConvention,
    numeric_hotkeys: bool,
    delimiter: Option<char>,
) -> Result<Vec<ChoiceOption<String>>, NormalizeError> {
    let mut parsed: Vec<ParsedOption> = raw_options
        .into_iter()
        .map(|raw| raw_option_to_parsed(raw, delimiter))
        .collect::<Result<Vec<_>, _>>()?;

    if numeric_hotkeys {
        assign_numeric_hotkeys(&mut parsed);
    }

    // Check for duplicate hotkeys.
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
            let label = apply_convention(&opt.label, label_convention);
            let value = apply_convention(&opt.value, value_convention);
            // The id mirrors the (post-convention) value so callers
            // resolve `--selected` against the same string they will
            // see written back on submit.
            let id = value.clone();
            let mut choice = ChoiceOption::new(id, label, value).with_hotkey_option(opt.hotkey);
            if opt.disabled {
                choice = choice.disabled();
            }
            choice
        })
        .collect())
}

/// Lowers a [`RawOption`] into a [`ParsedOption`] applying the prefix
/// and delimiter rules with object-supplied fields taking precedence.
fn raw_option_to_parsed(
    raw: RawOption,
    delimiter: Option<char>,
) -> Result<ParsedOption, NormalizeError> {
    let raw_repr = render_raw_repr(&raw);

    // Hotkey: object-supplied wins outright; otherwise try to strip
    // a bracketed prefix from the label.
    let (hotkey, label_after_prefix) = if let Some(spec) = raw.hotkey.as_deref() {
        match parse_hotkey_spec(spec) {
            Some(h) => (Some(h), raw.label.clone()),
            None => {
                return Err(NormalizeError::InvalidHotkey {
                    spec: spec.to_string(),
                    option: raw.label.clone(),
                });
            }
        }
    } else {
        let (hotkey, rest) = extract_hotkey(&raw.label);
        (hotkey, rest.trim().to_string())
    };

    // Value: object-supplied wins outright; otherwise honour `::`,
    // then the legacy `--delimiter`. When neither is present the
    // value mirrors the label.
    let (label, value) = if let Some(value) = raw.value {
        (label_after_prefix, value)
    } else if let Some((lbl, val)) = label_after_prefix.split_once("::") {
        (lbl.trim().to_string(), val.trim().to_string())
    } else if let Some(delim) = delimiter {
        if let Some((lbl, val)) = label_after_prefix.split_once(delim) {
            (lbl.trim().to_string(), val.trim().to_string())
        } else {
            (label_after_prefix.clone(), label_after_prefix)
        }
    } else {
        (label_after_prefix.clone(), label_after_prefix)
    };

    Ok(ParsedOption {
        raw: raw_repr,
        label,
        value,
        hotkey,
        disabled: raw.disabled.unwrap_or(false),
    })
}

/// Builds a human-readable representation of a [`RawOption`] for use
/// in error messages (the `raw` field of [`ParsedOption`]).
fn render_raw_repr(raw: &RawOption) -> String {
    match (&raw.value, &raw.hotkey) {
        (Some(value), Some(hotkey)) => format!("[{}] {}::{}", hotkey, raw.label, value),
        (Some(value), None) => format!("{}::{}", raw.label, value),
        (None, Some(hotkey)) => format!("[{}] {}", hotkey, raw.label),
        (None, None) => raw.label.clone(),
    }
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

    fn raw(label: &str) -> RawOption {
        RawOption::from_label(label)
    }

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
    fn parse_hotkey_spec_canonical_forms() {
        assert_eq!(parse_hotkey_spec("CTRL+R"), Some(HotkeySpec::Ctrl('r')));
        assert_eq!(parse_hotkey_spec("ALT+B"), Some(HotkeySpec::Alt('b')));
        assert_eq!(parse_hotkey_spec("OPT+B"), Some(HotkeySpec::Alt('b')));
        assert_eq!(parse_hotkey_spec("ctrl+x"), Some(HotkeySpec::Ctrl('x')));
        assert_eq!(parse_hotkey_spec("nope"), None);
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
                disabled: false,
            },
            ParsedOption {
                raw: "b".into(),
                label: "b".into(),
                value: "b".into(),
                hotkey: None,
                disabled: false,
            },
        ];
        assign_numeric_hotkeys(&mut options);
        assert_eq!(options[0].hotkey, Some(HotkeySpec::Ctrl('1')));
        assert_eq!(options[1].hotkey, Some(HotkeySpec::Ctrl('2')));
    }

    #[test]
    fn numeric_hotkeys_tenth_is_ctrl_one() {
        let mut options = vec![ParsedOption {
            raw: "a".into(),
            label: "a".into(),
            value: "a".into(),
            hotkey: None,
            disabled: false,
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
                disabled: false,
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
            disabled: false,
        }];
        assign_numeric_hotkeys(&mut options);
        assert_eq!(options[0].hotkey, Some(HotkeySpec::Ctrl('x')));
    }

    #[test]
    fn normalize_options_basic() {
        let options = vec![raw("Apple"), raw("Berry")];
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
        let options = vec![raw("hello world")];
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
        let options = vec![raw("[CTRL+x] A"), raw("[CTRL+x] B")];
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
        let options = vec![raw("Apple:1"), raw("Berry:2")];
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
        let options = vec![raw("Red::apple")];
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

    // --- Phase 3: object-record propagation tests --------------------

    #[test]
    fn normalize_options_preserves_object_value() {
        let options = vec![RawOption {
            label: "Red".into(),
            value: Some("apple".into()),
            hotkey: None,
            disabled: None,
        }];
        let result = normalize_options(
            options,
            NamingConvention::None,
            NamingConvention::None,
            false,
            None,
        )
        .unwrap();
        assert_eq!(result[0].label, "Red");
        assert_eq!(result[0].value, "apple");
        assert_eq!(result[0].id, "apple");
    }

    #[test]
    fn normalize_options_preserves_object_hotkey() {
        let options = vec![RawOption {
            label: "Red".into(),
            value: Some("apple".into()),
            hotkey: Some("CTRL+R".into()),
            disabled: None,
        }];
        let result = normalize_options(
            options,
            NamingConvention::None,
            NamingConvention::None,
            false,
            None,
        )
        .unwrap();
        assert_eq!(result[0].hotkey, Some(HotkeySpec::Ctrl('r')));
    }

    #[test]
    fn normalize_options_preserves_object_disabled() {
        let options = vec![RawOption {
            label: "Red".into(),
            value: Some("apple".into()),
            hotkey: None,
            disabled: Some(true),
        }];
        let result = normalize_options(
            options,
            NamingConvention::None,
            NamingConvention::None,
            false,
            None,
        )
        .unwrap();
        assert!(result[0].disabled);
    }

    #[test]
    fn prefix_hotkey_does_not_overwrite_explicit_object_hotkey() {
        // Object supplies hotkey CTRL+R; the bracketed prefix in the
        // label must NOT take precedence (and must not be stripped
        // either, since the object author chose to embed brackets in
        // their label).
        let options = vec![RawOption {
            label: "[CTRL+x] Red".into(),
            value: Some("apple".into()),
            hotkey: Some("CTRL+R".into()),
            disabled: None,
        }];
        let result = normalize_options(
            options,
            NamingConvention::None,
            NamingConvention::None,
            false,
            None,
        )
        .unwrap();
        assert_eq!(result[0].hotkey, Some(HotkeySpec::Ctrl('r')));
        // The bracketed prefix stays in the label because the object's
        // explicit hotkey field wins, which means the prefix parser is
        // never consulted.
        assert_eq!(result[0].label, "[CTRL+x] Red");
    }

    #[test]
    fn delimiter_split_does_not_overwrite_explicit_object_value() {
        // Object supplies value=apple; even though the label contains
        // `::`, the object value wins and the label is not split.
        let options = vec![RawOption {
            label: "Red::not-this".into(),
            value: Some("apple".into()),
            hotkey: None,
            disabled: None,
        }];
        let result = normalize_options(
            options,
            NamingConvention::None,
            NamingConvention::None,
            false,
            Some(':'),
        )
        .unwrap();
        assert_eq!(result[0].label, "Red::not-this");
        assert_eq!(result[0].value, "apple");
    }

    #[test]
    fn numeric_hot_keys_skips_options_with_explicit_hotkey() {
        let options = vec![
            RawOption {
                label: "Red".into(),
                value: Some("r".into()),
                hotkey: Some("CTRL+R".into()),
                disabled: None,
            },
            RawOption::from_label("Blue"),
        ];
        let result = normalize_options(
            options,
            NamingConvention::None,
            NamingConvention::None,
            true,
            None,
        )
        .unwrap();
        assert_eq!(result[0].hotkey, Some(HotkeySpec::Ctrl('r')));
        // Second option gets numeric Ctrl+1 (not Ctrl+2) because we
        // assign numerically by index, then skip filled slots — so
        // index 1 lands on Ctrl+2.
        assert_eq!(result[1].hotkey, Some(HotkeySpec::Ctrl('2')));
    }

    #[test]
    fn invalid_object_hotkey_returns_error() {
        let options = vec![RawOption {
            label: "Red".into(),
            value: None,
            hotkey: Some("not-a-hotkey".into()),
            disabled: None,
        }];
        let result = normalize_options(
            options,
            NamingConvention::None,
            NamingConvention::None,
            false,
            None,
        );
        assert!(matches!(result, Err(NormalizeError::InvalidHotkey { .. })));
    }
}
