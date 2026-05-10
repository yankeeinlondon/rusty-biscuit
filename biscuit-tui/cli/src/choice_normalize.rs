//! Normalization of typed [`RawOption`] records into [`ChoiceOption`] values.
//!
//! Handles hotkey prefix parsing, label/value convention transforms,
//! `::` delimiter splitting, and numeric hotkey assignment. Object
//! sources (JSON/YAML/TOML/CSV/markdown frontmatter object arrays)
//! flow through with their explicit label, `value`, and `hotkey`
//! preserved; string sources fall back to the legacy prefix/delimiter
//! parsing.

use heck::{ToKebabCase, ToLowerCamelCase, ToSnakeCase, ToTitleCase, ToUpperCamelCase};
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
    pub explicit_label: bool,
    pub explicit_value: bool,
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
            explicit_label: true,
            explicit_value: true,
        }
    } else {
        ParsedOption {
            raw: raw.to_string(),
            label: rest.to_string(),
            value: rest.to_string(),
            hotkey,
            disabled: false,
            explicit_label: false,
            explicit_value: false,
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
/// prefix, when the suffix after the modifier is empty, or when the
/// suffix contains more than a single character. Object sources that
/// supply a `hotkey` field route through this same parser so the wire
/// format matches the bracketed prefix form (`[CTRL+R]`).
pub fn parse_hotkey_spec(spec: &str) -> Option<HotkeySpec> {
    let spec = spec.trim();
    let upper = spec.to_uppercase();
    if let Some(rest) = upper.strip_prefix("CTRL+") {
        let ch = single_char(rest)?;
        return Some(HotkeySpec::Ctrl(ch.to_ascii_lowercase()));
    }
    if let Some(rest) = upper.strip_prefix("ALT+") {
        let ch = single_char(rest)?;
        return Some(HotkeySpec::Alt(ch.to_ascii_lowercase()));
    }
    if let Some(rest) = upper.strip_prefix("OPT+") {
        let ch = single_char(rest)?;
        return Some(HotkeySpec::Alt(ch.to_ascii_lowercase()));
    }
    None
}

/// Returns `Some(ch)` when `rest` contains exactly one Unicode scalar
/// value, otherwise returns `None`.
///
/// Used by [`parse_hotkey_spec`] to enforce the single-character contract
/// for hotkey suffixes after `CTRL+`, `ALT+`, or `OPT+` prefixes.
fn single_char(rest: &str) -> Option<char> {
    let mut iter = rest.chars();
    let ch = iter.next()?;
    if iter.next().is_some() {
        return None;
    }
    Some(ch)
}

/// Returns `true` when `bracket_contents` (case-insensitively) starts
/// with one of the recognized modifier prefixes (`CTRL+`, `ALT+`, or
/// `OPT+`).
///
/// Used to distinguish "no recognized hotkey prefix at all" from "a
/// recognized modifier prefix with an invalid suffix" in bracketed CLI
/// labels such as `[CTRL+RED] Red`.
fn has_modifier_prefix(bracket_contents: &str) -> bool {
    let upper = bracket_contents.trim().to_uppercase();
    upper.starts_with("CTRL+") || upper.starts_with("ALT+") || upper.starts_with("OPT+")
}

/// Extracts the contents of a leading `[...]` bracket from `s` if
/// present, returning `Some((bracket_contents, after))` where `after`
/// is the substring following the closing `]`.
///
/// Returns `None` when `s` does not begin (after leading whitespace)
/// with a `[` followed by a `]`.
fn split_bracket_prefix(s: &str) -> Option<(&str, &str)> {
    let trimmed = s.trim_start();
    let rest = trimmed.strip_prefix('[')?;
    let end_idx = rest.find(']')?;
    Some((&rest[..end_idx], &rest[end_idx + 1..]))
}

/// Applies a naming convention to a string.
pub fn apply_convention(s: &str, convention: NamingConvention) -> String {
    match convention {
        NamingConvention::None => s.to_string(),
        NamingConvention::CamelCase => s.to_lower_camel_case(),
        NamingConvention::PascalCase => s.to_upper_camel_case(),
        NamingConvention::KebabCase => s.to_kebab_case(),
        NamingConvention::SnakeCase => s.to_snake_case(),
        NamingConvention::TitleCase => s.to_title_case(),
        NamingConvention::Caps => s.to_uppercase(),
        NamingConvention::Lowercase => s.to_lowercase(),
    }
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

/// Renders a [`HotkeySpec`] in the user-facing chord form (`Ctrl+r`,
/// `Alt+b`) used in [`NormalizeError::DuplicateHotkey`] messages.
fn format_hotkey_spec(hotkey: HotkeySpec) -> String {
    match hotkey {
        HotkeySpec::Ctrl(c) => format!("Ctrl+{}", c),
        HotkeySpec::Alt(c) => format!("Alt+{}", c),
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
///    only to sides that were inferred from plain string options.
///    Object-supplied fields, `::` splits, and legacy delimiter splits
///    are explicit and are not convention-transformed.
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

    resolve_hotkey_collisions(&parsed)?;

    Ok(parsed
        .into_iter()
        .map(|opt| {
            let label = if opt.explicit_label {
                opt.label
            } else {
                apply_convention(&opt.label, label_convention)
            };
            let value = if opt.explicit_value {
                opt.value
            } else {
                apply_convention(&opt.value, value_convention)
            };
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

/// Validates hotkey collisions across `parsed`.
///
/// The only collision shape that errors is **explicit-vs-explicit**:
/// two options that each carry a user-supplied (or numeric-assigned)
/// hotkey on the same chord. Options without an explicit hotkey
/// contribute no badge and no keypress binding.
///
/// Disabled options contribute no effective hotkey and participate in
/// no collisions.
fn resolve_hotkey_collisions(parsed: &[ParsedOption]) -> Result<(), NormalizeError> {
    let mut owners: std::collections::HashMap<HotkeySpec, usize> = std::collections::HashMap::new();
    for (idx, option) in parsed.iter().enumerate() {
        if option.disabled {
            continue;
        }
        if let Some(hotkey) = option.hotkey {
            if let Some(&first_idx) = owners.get(&hotkey) {
                return Err(NormalizeError::DuplicateHotkey {
                    hotkey: format_hotkey_spec(hotkey),
                    first: parsed[first_idx].raw.clone(),
                    second: option.raw.clone(),
                });
            }
            owners.insert(hotkey, idx);
        }
    }
    Ok(())
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
    } else if let Some((bracket_contents, after)) = split_bracket_prefix(&raw.label) {
        match parse_hotkey_spec(bracket_contents) {
            Some(h) => (Some(h), after.trim().to_string()),
            None if has_modifier_prefix(bracket_contents) => {
                // A recognized modifier prefix with an invalid suffix
                // (empty, multi-character, or otherwise unparseable)
                // must surface as an error rather than silently leaving
                // the bracket text in the label.
                return Err(NormalizeError::InvalidHotkey {
                    spec: bracket_contents.to_string(),
                    option: raw.label.clone(),
                });
            }
            None => (None, raw.label.trim().to_string()),
        }
    } else {
        (None, raw.label.trim().to_string())
    };

    // Value: object-supplied wins outright; otherwise honour `::`,
    // then the legacy `--delimiter`. When neither is present the
    // value mirrors the label.
    let (label, value, explicit_label, explicit_value) = if let Some(value) = raw.value {
        (label_after_prefix, value, true, true)
    } else if let Some((lbl, val)) = label_after_prefix.split_once("::") {
        (lbl.trim().to_string(), val.trim().to_string(), true, true)
    } else if let Some(delim) = delimiter {
        if let Some((lbl, val)) = label_after_prefix.split_once(delim) {
            (lbl.trim().to_string(), val.trim().to_string(), true, true)
        } else {
            (label_after_prefix.clone(), label_after_prefix, false, false)
        }
    } else {
        (label_after_prefix.clone(), label_after_prefix, false, false)
    };

    Ok(ParsedOption {
        raw: raw_repr,
        label,
        value,
        hotkey,
        disabled: raw.disabled.unwrap_or(false),
        explicit_label,
        explicit_value,
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
    fn parse_hotkey_spec_rejects_empty_suffix() {
        assert_eq!(parse_hotkey_spec("CTRL+"), None);
        assert_eq!(parse_hotkey_spec("ALT+"), None);
        assert_eq!(parse_hotkey_spec("OPT+"), None);
        assert_eq!(parse_hotkey_spec("ctrl+"), None);
        assert_eq!(parse_hotkey_spec("alt+"), None);
    }

    #[test]
    fn parse_hotkey_spec_rejects_multi_character_suffix() {
        assert_eq!(parse_hotkey_spec("CTRL+RED"), None);
        assert_eq!(parse_hotkey_spec("CTRL+AB"), None);
        assert_eq!(parse_hotkey_spec("ALT+AB"), None);
        assert_eq!(parse_hotkey_spec("OPT+AB"), None);
        // Mixed-case multi-character suffix.
        assert_eq!(parse_hotkey_spec("ctrl+Red"), None);
        assert_eq!(parse_hotkey_spec("Opt+ab"), None);
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
                explicit_label: false,
                explicit_value: false,
            },
            ParsedOption {
                raw: "b".into(),
                label: "b".into(),
                value: "b".into(),
                hotkey: None,
                disabled: false,
                explicit_label: false,
                explicit_value: false,
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
            explicit_label: false,
            explicit_value: false,
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
                explicit_label: false,
                explicit_value: false,
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
            explicit_label: false,
            explicit_value: false,
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
    fn normalize_options_delimited_value_skips_value_convention() {
        let options = vec![raw("Red Delicious::Apple")];
        let result = normalize_options(
            options,
            NamingConvention::None,
            NamingConvention::SnakeCase,
            false,
            None,
        )
        .unwrap();
        assert_eq!(result[0].label, "Red Delicious");
        assert_eq!(result[0].value, "Apple");
        assert_eq!(result[0].id, "Apple");
    }

    #[test]
    fn normalize_options_delimited_label_skips_label_convention() {
        let options = vec![raw("red delicious::Apple")];
        let result = normalize_options(
            options,
            NamingConvention::TitleCase,
            NamingConvention::None,
            false,
            None,
        )
        .unwrap();
        assert_eq!(result[0].label, "red delicious");
        assert_eq!(result[0].value, "Apple");
    }

    #[test]
    fn normalize_options_object_value_skips_value_convention() {
        let options = vec![RawOption {
            label: "Red Delicious".into(),
            value: Some("Apple".into()),
            hotkey: None,
            disabled: None,
        }];
        let result = normalize_options(
            options,
            NamingConvention::None,
            NamingConvention::SnakeCase,
            false,
            None,
        )
        .unwrap();
        assert_eq!(result[0].label, "Red Delicious");
        assert_eq!(result[0].value, "Apple");
        assert_eq!(result[0].id, "Apple");
    }

    #[test]
    fn normalize_options_legacy_delimiter_skips_conventions() {
        let options = vec![raw("red delicious:Apple")];
        let result = normalize_options(
            options,
            NamingConvention::TitleCase,
            NamingConvention::SnakeCase,
            false,
            Some(':'),
        )
        .unwrap();
        assert_eq!(result[0].label, "red delicious");
        assert_eq!(result[0].value, "Apple");
        assert_eq!(result[0].id, "Apple");
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
        match result {
            Err(NormalizeError::DuplicateHotkey {
                hotkey,
                first,
                second,
            }) => {
                assert_eq!(hotkey, "Ctrl+x");
                assert_eq!(first, "[CTRL+x] A");
                assert_eq!(second, "[CTRL+x] B");
            }
            other => panic!("expected DuplicateHotkey, got {other:?}"),
        }
    }

    #[test]
    fn unassigned_options_have_no_hotkey() {
        // Auto-derivation from the first label character is gone.
        // An option without an explicit `[CTRL+x]` prefix gets no
        // hotkey at all — no badge, no keypress binding. The user
        // who did not opt in to a hotkey will not see one.
        let options = vec![raw("Red"), raw("[CTRL+R] Rose")];
        let result = normalize_options(
            options,
            NamingConvention::None,
            NamingConvention::None,
            false,
            None,
        )
        .expect("plain options without explicit hotkeys never collide");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].label, "Red");
        assert_eq!(result[0].effective_hotkey(), None);
        assert_eq!(result[1].label, "Rose");
        assert_eq!(result[1].effective_hotkey(), Some(HotkeySpec::Ctrl('r')));
    }

    #[test]
    fn three_plain_labels_run_with_no_badges() {
        // `bar`, `baz`, `bax` are plain labels with no explicit
        // hotkeys — none of them gets a badge. The CLI runs cleanly
        // and no "duplicate hotkey" error fires.
        let options = vec![raw("bar"), raw("baz"), raw("bax")];
        let result = normalize_options(
            options,
            NamingConvention::None,
            NamingConvention::None,
            false,
            None,
        )
        .expect("plain labels never produce collisions");
        assert_eq!(result[0].effective_hotkey(), None);
        assert_eq!(result[1].effective_hotkey(), None);
        assert_eq!(result[2].effective_hotkey(), None);
    }

    #[test]
    fn user_reported_scenario_one_explicit_three_plain() {
        // `question choose-one "[CTRL+f]foo" bar baz bax`:
        //   - `foo` has the explicit `Ctrl+F` hotkey
        //   - `bar`, `baz`, `bax` are plain labels with no hotkey
        // The user wrote one hotkey; one hotkey appears.
        let options = vec![raw("[CTRL+f]foo"), raw("bar"), raw("baz"), raw("bax")];
        let result = normalize_options(
            options,
            NamingConvention::None,
            NamingConvention::None,
            false,
            None,
        )
        .expect("only user-set explicit hotkeys can produce errors");
        assert_eq!(result.len(), 4);
        assert_eq!(result[0].effective_hotkey(), Some(HotkeySpec::Ctrl('f')));
        assert_eq!(result[1].effective_hotkey(), None);
        assert_eq!(result[2].effective_hotkey(), None);
        assert_eq!(result[3].effective_hotkey(), None);
    }

    #[test]
    fn normalize_disabled_implicit_does_not_collide() {
        // The first option is disabled, so it contributes no effective
        // hotkey. The second option's implicit `Ctrl+r` therefore has
        // no collision and the normalization should succeed.
        let options = vec![
            RawOption {
                label: "Red".into(),
                value: None,
                hotkey: None,
                disabled: Some(true),
            },
            RawOption::from_label("Rose"),
        ];
        let result = normalize_options(
            options,
            NamingConvention::None,
            NamingConvention::None,
            false,
            None,
        )
        .expect("disabled options should not contribute an effective hotkey");
        assert_eq!(result.len(), 2);
        assert!(result[0].disabled);
        assert_eq!(result[0].label, "Red");
        assert_eq!(result[1].label, "Rose");
    }

    #[test]
    fn normalize_numeric_hotkey_collision_with_explicit_ctrl_one_is_rejected() {
        // The first option carries an explicit `[CTRL+1]`, and
        // `assign_numeric_hotkeys` would also assign `Ctrl+1` to the
        // first hotkey-less option (here the second). Numeric hotkeys
        // run before the duplicate check, so the second option ends up
        // with an effective `Ctrl+1` colliding with the first option's
        // explicit binding.
        //
        // Note: `assign_numeric_hotkeys` walks by index, so when
        // index 0 already has an explicit hotkey, index 1 receives
        // `Ctrl+2`. To reliably trigger the collision we put the
        // explicit `[CTRL+1]` on a later option so an earlier slot
        // claims `Ctrl+1` first via the numeric pass.
        let options = vec![raw("Apple"), raw("[CTRL+1] Banana")];
        let result = normalize_options(
            options,
            NamingConvention::None,
            NamingConvention::None,
            true,
            None,
        );
        match result {
            Err(NormalizeError::DuplicateHotkey { hotkey, .. }) => {
                assert_eq!(hotkey, "Ctrl+1");
            }
            other => panic!("expected DuplicateHotkey on Ctrl+1, got {other:?}"),
        }
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

    #[test]
    fn invalid_object_hotkey_multi_char_ctrl_returns_error() {
        let options = vec![RawOption {
            label: "Red".into(),
            value: None,
            hotkey: Some("CTRL+AB".into()),
            disabled: None,
        }];
        let result = normalize_options(
            options,
            NamingConvention::None,
            NamingConvention::None,
            false,
            None,
        );
        match result {
            Err(NormalizeError::InvalidHotkey { spec, option }) => {
                assert_eq!(spec, "CTRL+AB");
                assert_eq!(option, "Red");
            }
            other => panic!("expected InvalidHotkey, got {other:?}"),
        }
    }

    #[test]
    fn invalid_object_hotkey_empty_alt_returns_error() {
        let options = vec![RawOption {
            label: "Red".into(),
            value: None,
            hotkey: Some("ALT+".into()),
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

    #[test]
    fn invalid_object_hotkey_multi_char_opt_returns_error() {
        let options = vec![RawOption {
            label: "Red".into(),
            value: None,
            hotkey: Some("OPT+AB".into()),
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

    #[test]
    fn invalid_bracketed_multi_char_ctrl_returns_error() {
        let options = vec![raw("[CTRL+AB] Red")];
        let result = normalize_options(
            options,
            NamingConvention::None,
            NamingConvention::None,
            false,
            None,
        );
        match result {
            Err(NormalizeError::InvalidHotkey { spec, option }) => {
                assert_eq!(spec, "CTRL+AB");
                assert_eq!(option, "[CTRL+AB] Red");
            }
            other => panic!("expected InvalidHotkey, got {other:?}"),
        }
    }

    #[test]
    fn invalid_bracketed_empty_alt_returns_error() {
        let options = vec![raw("[ALT+] Red")];
        let result = normalize_options(
            options,
            NamingConvention::None,
            NamingConvention::None,
            false,
            None,
        );
        assert!(matches!(result, Err(NormalizeError::InvalidHotkey { .. })));
    }

    #[test]
    fn invalid_bracketed_multi_char_opt_returns_error() {
        let options = vec![raw("[OPT+AB] Red")];
        let result = normalize_options(
            options,
            NamingConvention::None,
            NamingConvention::None,
            false,
            None,
        );
        assert!(matches!(result, Err(NormalizeError::InvalidHotkey { .. })));
    }

    #[test]
    fn valid_bracketed_ctrl_still_normalizes() {
        let options = vec![raw("[CTRL+R] Red")];
        let result = normalize_options(
            options,
            NamingConvention::None,
            NamingConvention::None,
            false,
            None,
        )
        .unwrap();
        assert_eq!(result[0].label, "Red");
        assert_eq!(result[0].hotkey, Some(HotkeySpec::Ctrl('r')));
    }

    #[test]
    fn non_modifier_bracket_prefix_is_treated_as_label_text() {
        // `[note]` is not a recognized modifier prefix, so it must be
        // treated as ordinary label content rather than an error.
        let options = vec![raw("[note] Red")];
        let result = normalize_options(
            options,
            NamingConvention::None,
            NamingConvention::None,
            false,
            None,
        )
        .unwrap();
        assert!(result[0].hotkey.is_none());
        assert_eq!(result[0].label, "[note] Red");
    }
}
