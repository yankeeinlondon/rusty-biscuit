//! Reading resolved configuration values out of a terminal's config file.
//!
//! Once the resolver ([`super::resolver`]) has found the file *in use* on this
//! host, this module extracts the raw value each [`SettingLocator`] points at.
//! Extraction is best-effort and format-bounded (the central honesty of the
//! design, spec §6): metadata is always reportable, but a value is only
//! recovered when the format is statically parseable.
//!
//! Strategy per [`ConfigFormat`]:
//!
//! - `Toml` / `Yaml` / `Json` / `Json5` — normalized through `biscuit-file` to a
//!   single [`serde_json::Value`] and read by one shared dot-path resolver.
//! - `Plist` — parsed by the `plist` crate (XML *and* binary) and read by a
//!   sibling dot-path resolver over [`plist::Value`].
//! - `KittyConf` / `KeyValue` — flat line scan (last value wins), with
//!   best-effort `include` / `config-file` following.
//! - Directory candidates — locator-only; v1 does not choose one child file out
//!   of a vendor-managed directory.
//! - `Lua` / `Dconf` / `None` — locator-only; no value is extracted and a short
//!   machine-readable reason is reported instead.
//!
//! v1 returns **raw** values only; [`SettingLocator::value_kind`] is advisory
//! display metadata and never coerces or validates the extracted value.

use std::collections::HashMap;
use std::path::Path;

use serde::Serialize;

use super::types::ConfigFormat;

/// The outcome of extracting one setting value from a resolved config file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SettingValue {
    /// A raw value was found at the locator.
    Found {
        /// The value exactly as it appears in the file (no normalization).
        value: String,
    },
    /// The file parsed but the locator's key is not present.
    Absent,
    /// The format is not statically extractable in v1 (Lua/Dconf/None).
    ///
    /// `reason` is a short machine-readable token (e.g. `"Lua"`).
    LocatorOnly {
        /// Why no value was extracted.
        reason: &'static str,
    },
    /// The file could not be read or parsed.
    Unreadable {
        /// A human-readable description of the read/parse failure.
        reason: String,
    },
    /// The locator is known, but no config file was available to read.
    Unavailable {
        /// Why no value was extracted.
        reason: &'static str,
        /// Always serialized as JSON null to keep the value field explicit.
        value: (),
    },
}

/// A parsed terminal config file, ready for repeated setting extraction.
///
/// Loaded once via [`ConfigDocument::load`], then queried per locator with
/// [`ConfigDocument::extract`] — the file is read and parsed a single time even
/// when many settings are pulled from it.
pub enum ConfigDocument {
    /// Structured formats (TOML/YAML/JSON/JSON5) normalized to one JSON value.
    Structured(serde_json::Value),
    /// An Apple property list (XML or binary).
    Plist(plist::Value),
    /// Flat `key value` / `key = value` lines, last-value-wins.
    Flat(HashMap<String, String>),
    /// A format v1 does not statically extract, carrying its reason.
    LocatorOnly(&'static str),
    /// The file could not be read or parsed.
    Unreadable(String),
}

/// How a flat config file separates keys from values.
#[derive(Debug, Clone, Copy)]
enum FlatStyle {
    /// `key value` (kitty.conf).
    SpaceSeparated,
    /// `key = value` (ghostty, foot/ini-ish).
    Equals,
}

/// Guard against runaway `include` recursion / cycles in flat configs.
const MAX_INCLUDE_DEPTH: usize = 8;

impl ConfigDocument {
    /// Load and parse `path` according to `format`.
    ///
    /// Never panics: read/parse failures are captured as
    /// [`ConfigDocument::Unreadable`], and formats without a v1 extractor become
    /// [`ConfigDocument::LocatorOnly`].
    pub fn load(path: &Path, format: ConfigFormat) -> ConfigDocument {
        if path.is_dir() {
            return ConfigDocument::LocatorOnly("directory candidate");
        }

        match format {
            ConfigFormat::Toml => load_structured(path, StructuredKind::Toml),
            ConfigFormat::Yaml => load_structured(path, StructuredKind::Yaml),
            ConfigFormat::Json | ConfigFormat::Json5 => load_structured(path, StructuredKind::Json5),
            ConfigFormat::Plist => load_plist(path),
            ConfigFormat::KittyConf => load_flat(path, FlatStyle::SpaceSeparated),
            ConfigFormat::KeyValue => load_flat(path, FlatStyle::Equals),
            ConfigFormat::Lua => ConfigDocument::LocatorOnly("Lua"),
            ConfigFormat::Dconf => ConfigDocument::LocatorOnly("Dconf"),
            ConfigFormat::None => ConfigDocument::LocatorOnly("no parseable config"),
        }
    }

    /// Extract the raw value at `locator_path`.
    ///
    /// The path is interpreted per the format the document was loaded with:
    /// nested dot path for structured/plist documents, a verbatim flat key for
    /// flat documents. See [`resolve_json_path`] for the structured-key fallback
    /// that also handles dotted flat keys (VS Code style).
    pub fn extract(&self, locator_path: &str) -> SettingValue {
        match self {
            ConfigDocument::Structured(value) => match resolve_json_path(value, locator_path) {
                Some(leaf) => SettingValue::Found {
                    value: render_json_leaf(leaf),
                },
                None => SettingValue::Absent,
            },
            ConfigDocument::Plist(value) => match resolve_plist_path(value, locator_path) {
                Some(leaf) => SettingValue::Found {
                    value: render_plist_leaf(leaf),
                },
                None => SettingValue::Absent,
            },
            ConfigDocument::Flat(map) => match map.get(locator_path) {
                Some(value) => SettingValue::Found {
                    value: value.clone(),
                },
                None => SettingValue::Absent,
            },
            ConfigDocument::LocatorOnly(reason) => SettingValue::LocatorOnly { reason },
            ConfigDocument::Unreadable(reason) => SettingValue::Unreadable {
                reason: reason.clone(),
            },
        }
    }
}

/// Which `biscuit-file` parser normalizes a structured format to JSON.
#[derive(Debug, Clone, Copy)]
enum StructuredKind {
    Toml,
    Yaml,
    /// Plain JSON is a subset of JSON5, so both route through the JSON5 parser.
    Json5,
}

/// Parse a structured config file into one [`serde_json::Value`] via `biscuit-file`.
fn load_structured(path: &Path, kind: StructuredKind) -> ConfigDocument {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) => return ConfigDocument::Unreadable(err.to_string()),
    };
    let value = match kind {
        StructuredKind::Toml => biscuit_file::Toml::from_str(&text)
            .and_then(|doc| doc.as_json_value())
            .map_err(|err| err.to_string()),
        StructuredKind::Yaml => biscuit_file::Yaml::from_str(&text)
            .and_then(|doc| doc.as_json())
            .map_err(|err| err.to_string()),
        StructuredKind::Json5 => biscuit_file::Json5::from_str(&text)
            .map(|doc| doc.as_json_value().clone())
            .map_err(|err| err.to_string()),
    };
    match value {
        Ok(value) => ConfigDocument::Structured(value),
        Err(reason) => ConfigDocument::Unreadable(reason),
    }
}

/// Parse an Apple property list (XML or binary) with the `plist` crate.
fn load_plist(path: &Path) -> ConfigDocument {
    match plist::Value::from_file(path) {
        Ok(value) => ConfigDocument::Plist(value),
        Err(err) => ConfigDocument::Unreadable(err.to_string()),
    }
}

/// Scan a flat config file into a last-value-wins key/value map.
fn load_flat(path: &Path, style: FlatStyle) -> ConfigDocument {
    let mut map = HashMap::new();
    if let Err(reason) = scan_flat(path, style, &mut map, 0) {
        return ConfigDocument::Unreadable(reason);
    }
    ConfigDocument::Flat(map)
}

/// Scan one flat file into `map`, following `include` directives inline.
///
/// Inline following matches kitty/ghostty semantics: an included file's keys
/// take effect where the directive appears, so later definitions in the parent
/// still override them. Includes past [`MAX_INCLUDE_DEPTH`] are skipped rather
/// than errored (cycle guard).
fn scan_flat(
    path: &Path,
    style: FlatStyle,
    map: &mut HashMap<String, String>,
    depth: usize,
) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|err| err.to_string())?;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') || line.starts_with('[')
        {
            continue;
        }

        if let Some(target) = include_target(line, style) {
            if depth < MAX_INCLUDE_DEPTH
                && let Some(resolved) = resolve_include(path, target)
            {
                // A missing/unreadable include is ignored (best-effort).
                let _ = scan_flat(&resolved, style, map, depth + 1);
            }
            continue;
        }

        let separated = match style {
            FlatStyle::SpaceSeparated => line.split_once(char::is_whitespace),
            FlatStyle::Equals => line.split_once('='),
        };
        if let Some((key, value)) = separated {
            map.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    Ok(())
}

/// The include-target path from a flat line, if it is an include directive.
///
/// Recognizes kitty's `include <path>` and ghostty's `config-file = <path>`.
fn include_target(line: &str, style: FlatStyle) -> Option<&str> {
    match style {
        FlatStyle::SpaceSeparated => line
            .strip_prefix("include ")
            .map(str::trim)
            .filter(|target| !target.is_empty()),
        FlatStyle::Equals => line
            .split_once('=')
            .filter(|(key, _)| key.trim() == "config-file")
            .map(|(_, value)| value.trim())
            .filter(|target| !target.is_empty()),
    }
}

/// Resolve an include target relative to the including file's directory.
///
/// Absolute targets are used as-is. `~`-prefixed targets are not expanded here
/// (best-effort v1); such includes are simply skipped.
fn resolve_include(parent: &Path, target: &str) -> Option<std::path::PathBuf> {
    if target.starts_with('~') {
        return None;
    }
    let candidate = Path::new(target);
    if candidate.is_absolute() {
        return Some(candidate.to_path_buf());
    }
    Some(parent.parent()?.join(candidate))
}

/// Resolve a dot path against a JSON value, tolerating dotted flat keys.
///
/// At each level the *entire remaining* path is first tried as a literal object
/// key — this recovers VS Code-style settings where `terminal.integrated.fontSize`
/// is one flat key rather than nested objects. Failing that, the first segment is
/// split off and used to descend (object key, or array index for numeric
/// segments), then the remainder resolves recursively.
fn resolve_json_path<'a>(
    current: &'a serde_json::Value,
    path: &str,
) -> Option<&'a serde_json::Value> {
    if let serde_json::Value::Object(map) = current
        && let Some(found) = map.get(path)
    {
        return Some(found);
    }
    let (head, rest) = split_head(path);
    let next = match current {
        serde_json::Value::Object(map) => map.get(head)?,
        serde_json::Value::Array(items) => items.get(head.parse::<usize>().ok()?)?,
        _ => return None,
    };
    match rest {
        Some(rest) => resolve_json_path(next, rest),
        None => Some(next),
    }
}

/// Resolve a dot path against a plist value (dict keys, array indices).
fn resolve_plist_path<'a>(current: &'a plist::Value, path: &str) -> Option<&'a plist::Value> {
    let (head, rest) = split_head(path);
    let next = match current {
        plist::Value::Dictionary(dict) => dict.get(head)?,
        plist::Value::Array(items) => items.get(head.parse::<usize>().ok()?)?,
        _ => return None,
    };
    match rest {
        Some(rest) => resolve_plist_path(next, rest),
        None => Some(next),
    }
}

/// Split a dot path into its first segment and the (optional) remainder.
fn split_head(path: &str) -> (&str, Option<&str>) {
    match path.split_once('.') {
        Some((head, rest)) => (head, Some(rest)),
        None => (path, None),
    }
}

/// Render a JSON leaf as its raw string (strings verbatim, scalars stringified,
/// containers as compact JSON).
fn render_json_leaf(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Bool(flag) => flag.to_string(),
        serde_json::Value::Number(number) => number.to_string(),
        serde_json::Value::Null => "null".to_string(),
        container => container.to_string(),
    }
}

/// Render a plist leaf as its raw string (scalars stringified; containers via `Debug`).
fn render_plist_leaf(value: &plist::Value) -> String {
    match value {
        plist::Value::String(text) => text.clone(),
        plist::Value::Integer(number) => number.to_string(),
        plist::Value::Real(number) => number.to_string(),
        plist::Value::Boolean(flag) => flag.to_string(),
        plist::Value::Uid(uid) => uid.get().to_string(),
        other => format!("{other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_fixture(name: &str, contents: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(name);
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(contents.as_bytes()).unwrap();
        (dir, path)
    }

    #[test]
    fn kitty_conf_extracts_flat_key_last_wins() {
        let (_dir, path) = write_fixture(
            "kitty.conf",
            "# a comment\n\nfont_family Fira Code\nfont_size 10.0\nfont_size 13.5\n",
        );
        let doc = ConfigDocument::load(&path, ConfigFormat::KittyConf);
        assert_eq!(
            doc.extract("font_family"),
            SettingValue::Found {
                value: "Fira Code".to_string()
            }
        );
        // Last definition wins.
        assert_eq!(
            doc.extract("font_size"),
            SettingValue::Found {
                value: "13.5".to_string()
            }
        );
        assert_eq!(doc.extract("background_opacity"), SettingValue::Absent);
    }

    #[test]
    fn kitty_conf_follows_include_inline() {
        let dir = tempfile::tempdir().unwrap();
        let included = dir.path().join("theme.conf");
        std::fs::write(&included, "background #000000\nfont_size 9\n").unwrap();
        let main = dir.path().join("kitty.conf");
        // The include lands before the parent's own font_size, so the parent wins.
        std::fs::write(&main, "include theme.conf\nfont_size 14\n").unwrap();

        let doc = ConfigDocument::load(&main, ConfigFormat::KittyConf);
        assert_eq!(
            doc.extract("background"),
            SettingValue::Found {
                value: "#000000".to_string()
            }
        );
        assert_eq!(
            doc.extract("font_size"),
            SettingValue::Found {
                value: "14".to_string()
            }
        );
    }

    #[test]
    fn key_value_extracts_equals_separated() {
        let (_dir, path) = write_fixture(
            "config",
            "; comment\nfont-family = JetBrains Mono\nbackground-opacity = 0.9\n",
        );
        let doc = ConfigDocument::load(&path, ConfigFormat::KeyValue);
        assert_eq!(
            doc.extract("font-family"),
            SettingValue::Found {
                value: "JetBrains Mono".to_string()
            }
        );
        assert_eq!(
            doc.extract("background-opacity"),
            SettingValue::Found {
                value: "0.9".to_string()
            }
        );
    }

    #[test]
    fn toml_extracts_nested_dot_path() {
        let (_dir, path) = write_fixture(
            "alacritty.toml",
            "[font]\nsize = 13.0\n\n[font.normal]\nfamily = \"Iosevka\"\n",
        );
        let doc = ConfigDocument::load(&path, ConfigFormat::Toml);
        assert_eq!(
            doc.extract("font.size"),
            SettingValue::Found {
                value: "13.0".to_string()
            }
        );
        assert_eq!(
            doc.extract("font.normal.family"),
            SettingValue::Found {
                value: "Iosevka".to_string()
            }
        );
        assert_eq!(doc.extract("font.missing"), SettingValue::Absent);
    }

    #[test]
    fn yaml_extracts_nested_dot_path() {
        let (_dir, path) = write_fixture(
            "contour.yml",
            "profiles:\n  main:\n    font:\n      size: 12\n",
        );
        let doc = ConfigDocument::load(&path, ConfigFormat::Yaml);
        assert_eq!(
            doc.extract("profiles.main.font.size"),
            SettingValue::Found {
                value: "12".to_string()
            }
        );
    }

    #[test]
    fn directory_candidates_are_locator_only() {
        let dir = tempfile::tempdir().unwrap();
        let doc = ConfigDocument::load(dir.path(), ConfigFormat::Yaml);

        assert_eq!(
            doc.extract("themes"),
            SettingValue::LocatorOnly {
                reason: "directory candidate"
            }
        );
    }

    #[test]
    fn json_extracts_nested_and_array_index() {
        let (_dir, path) = write_fixture(
            "settings.json",
            "{ \"profiles\": { \"defaults\": { \"font\": { \"size\": 11 } } }, \"list\": [\"a\", \"b\"] }",
        );
        let doc = ConfigDocument::load(&path, ConfigFormat::Json);
        assert_eq!(
            doc.extract("profiles.defaults.font.size"),
            SettingValue::Found {
                value: "11".to_string()
            }
        );
        assert_eq!(
            doc.extract("list.1"),
            SettingValue::Found {
                value: "b".to_string()
            }
        );
    }

    #[test]
    fn json5_tolerates_comments_and_trailing_commas() {
        let (_dir, path) = write_fixture(
            "settings.json",
            "{\n  // a comment\n  \"terminal.integrated.fontSize\": 14,\n}",
        );
        let doc = ConfigDocument::load(&path, ConfigFormat::Json5);
        // The dotted key is a single flat object key (VS Code style), not nested.
        assert_eq!(
            doc.extract("terminal.integrated.fontSize"),
            SettingValue::Found {
                value: "14".to_string()
            }
        );
    }

    #[test]
    fn plist_xml_extracts_nested_dot_path() {
        let (_dir, path) = write_fixture(
            "com.example.plist",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>New Bookmarks</key>
  <array>
    <dict>
      <key>Normal Font</key>
      <string>Menlo-Regular 12</string>
      <key>Scrollback Lines</key>
      <integer>5000</integer>
    </dict>
  </array>
</dict>
</plist>
"#,
        );
        let doc = ConfigDocument::load(&path, ConfigFormat::Plist);
        assert_eq!(
            doc.extract("New Bookmarks.0.Normal Font"),
            SettingValue::Found {
                value: "Menlo-Regular 12".to_string()
            }
        );
        assert_eq!(
            doc.extract("New Bookmarks.0.Scrollback Lines"),
            SettingValue::Found {
                value: "5000".to_string()
            }
        );
        assert_eq!(doc.extract("New Bookmarks.0.Missing"), SettingValue::Absent);
    }

    #[test]
    fn lua_is_locator_only() {
        let (_dir, path) = write_fixture("wezterm.lua", "return { font_size = 12 }\n");
        let doc = ConfigDocument::load(&path, ConfigFormat::Lua);
        assert_eq!(
            doc.extract("font_size"),
            SettingValue::LocatorOnly { reason: "Lua" }
        );
    }

    #[test]
    fn dconf_is_locator_only() {
        let doc = ConfigDocument::load(Path::new("/nonexistent"), ConfigFormat::Dconf);
        assert_eq!(
            doc.extract("font"),
            SettingValue::LocatorOnly { reason: "Dconf" }
        );
    }

    #[test]
    fn missing_structured_file_is_unreadable() {
        let doc = ConfigDocument::load(Path::new("/definitely/not/here.toml"), ConfigFormat::Toml);
        assert!(matches!(
            doc.extract("font.size"),
            SettingValue::Unreadable { .. }
        ));
    }
}
