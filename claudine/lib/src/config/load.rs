//! Reading one Claudine configuration file into a typed value.
//!
//! [`read_document`] loads `~/.claudine/config.yaml` or
//! `{repo-root}/.claudine/config.yaml` with the admitted-YAML loader, judges
//! it strictly against its scope's contract ([`super::schema`]): nothing is
//! converted, and every problem names the file, line, column, and JSON
//! pointer. The defaults the user file relies on are put in place by the
//! shared default engine, filling a setting that is absent or `null` alike,
//! and the result is what serde reads into
//! [`ClaudineConfig`](super::ClaudineConfig) or
//! [`RepoOverrideConfig`](super::RepoOverrideConfig). A repository's overrides
//! declare no default, so an absent or `null` one inherits. An action or route
//! still written in the flat form of earlier releases is refused with its
//! keyed spelling.
//!
//! `permissions` belongs to Darkmatter: it is validated here through the
//! imported type, but the typed value carries it only as written, never with
//! its defaults.
//!
//! User-facing guide: `claudine/docs/topics/configuration.md`.

use std::fmt;
use std::ops::Range;
use std::path::{Path, PathBuf};

use serde_json::Value;
use simplified_schema::yaml_documents::{LoadedYaml, load_yaml_document, locating_problem};
use simplified_schema::{AppliedDefault, SchemaError, ValidationFailure, ValidationProblem, ValidationProblemCode};

use super::schema::{ConfigScope, contract};
use crate::error::{ClaudineError, Result};

/// One located problem in a configuration document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigProblem {
    /// The file; `None` for a document that did not come from a file.
    pub file: Option<PathBuf>,
    /// The JSON pointer of the offending value (`/actions/session_start/0`);
    /// empty for the whole document.
    pub pointer: String,
    /// 1-based line, when the problem has a position in the text.
    pub line: Option<usize>,
    /// 1-based column in characters, when `line` is known.
    pub column: Option<usize>,
    /// What is wrong.
    pub message: String,
    /// Which kind of problem this is.
    pub kind: ConfigProblemKind,
}

/// What kind of problem a [`ConfigProblem`] is, for a caller that reports
/// it without its message (which can quote authored values).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ConfigProblemKind {
    /// A key the scope's schema does not declare. [`ConfigProblem::pointer`]
    /// names the key itself, and the line and column are the key's.
    UnknownKey,
    /// The file is not one admissible YAML (or JSON5) document, or not a
    /// mapping, so no key in it was judged.
    Syntax,
    /// A value the schema or a configuration rule refuses, or a file that
    /// cannot be read as configuration.
    InvalidSetting,
}

impl ConfigProblem {
    pub(crate) fn whole(file: Option<&Path>, kind: ConfigProblemKind, message: impl Into<String>) -> Self {
        Self {
            file: file.map(Path::to_path_buf),
            pointer: String::new(),
            line: None,
            column: None,
            message: message.into(),
            kind,
        }
    }

    /// The unknown key this problem names, for a
    /// [`ConfigProblemKind::UnknownKey`] problem.
    #[must_use]
    pub fn unknown_key(&self) -> Option<String> {
        if self.kind != ConfigProblemKind::UnknownKey {
            return None;
        }
        simplified_schema::JsonPointer::parse(&self.pointer).segments().last().cloned()
    }
}

impl fmt::Display for ConfigProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(file) = &self.file {
            write!(f, "{}", file.display())?;
            if let (Some(line), Some(column)) = (self.line, self.column) {
                write!(f, ":{line}:{column}")?;
            }
            if !self.pointer.is_empty() {
                write!(f, " at `{}`", self.pointer)?;
            }
            write!(f, ": {}", self.message)
        } else if self.pointer.is_empty() {
            f.write_str(&self.message)
        } else {
            write!(f, "`{}`: {}", self.pointer, self.message)
        }
    }
}

/// Joins problems for an error message, one per line after the first.
pub(crate) fn join_problems(problems: &[ConfigProblem]) -> String {
    problems.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n")
}

/// One configuration file, judged and with its defaults in place.
#[derive(Debug, Clone)]
pub struct ConfigDocument {
    /// The file read.
    pub path: PathBuf,
    /// Which scope's contract judged it.
    pub scope: ConfigScope,
    /// The document as written.
    pub authored: Value,
    /// The document with every default the schema supplies in place, and
    /// `permissions` exactly as written. This is what serde reads.
    pub effective: Value,
    /// The defaults [`Self::effective`] holds, parents before children.
    pub defaults: Vec<AppliedDefault>,
}

/// Reads the configuration file at `path` as a document of `scope`.
///
/// ## Errors
///
/// - [`ClaudineError::ConfigNotFound`] when nothing is at `path`;
/// - [`ClaudineError::Io`] when it cannot be read;
/// - [`ClaudineError::ConfigInvalid`] when it is not UTF-8, not one
///   admissible YAML document, not a mapping, or a value its scope's schema
///   refuses; each problem is located.
pub fn read_document(path: &Path, scope: ConfigScope) -> Result<ConfigDocument> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(ClaudineError::ConfigNotFound(path.to_path_buf()));
        }
        Err(error) => return Err(ClaudineError::Io(error)),
    };
    document_from_bytes(&bytes, path, scope)
}

/// [`read_document`] over bytes already read from `path`.
pub(crate) fn document_from_bytes(bytes: &[u8], path: &Path, scope: ConfigScope) -> Result<ConfigDocument> {
    let invalid = |problems: Vec<ConfigProblem>| ClaudineError::ConfigInvalid { problems };
    let text = std::str::from_utf8(bytes).map_err(|error| {
        let text = String::from_utf8_lossy(bytes);
        let (line, column) = line_column(&text, error.valid_up_to());
        invalid(vec![ConfigProblem {
            file: Some(path.to_path_buf()),
            pointer: String::new(),
            line: Some(line),
            column: Some(column),
            message: "the file is not valid UTF-8".into(),
            kind: ConfigProblemKind::Syntax,
        }])
    })?;
    let loaded = load_yaml_document(text, path).map_err(|error| {
        let (message, span) = match error {
            SchemaError::SchemaDocument { message, span, .. } => (message, span),
            other => (other.to_string(), None),
        };
        let at = span.map(|span| line_column(text, span.start));
        invalid(vec![ConfigProblem {
            file: Some(path.to_path_buf()),
            pointer: String::new(),
            line: at.map(|(line, _)| line),
            column: at.map(|(_, column)| column),
            message,
            kind: ConfigProblemKind::Syntax,
        }])
    })?;
    let locator = Locator { path, text, loaded: &loaded };
    let (effective, defaults) =
        project(scope, &loaded.json, &|segments, kind, message| locator.problem(segments, kind, message))
            .map_err(invalid)?;
    Ok(ConfigDocument { path: path.to_path_buf(), scope, authored: loaded.json.clone(), effective, defaults })
}

/// Judges a whole document of `scope` that came from no file.
///
/// ## Errors
///
/// [`ClaudineError::ConfigInvalid`] with each problem's JSON pointer.
pub(crate) fn project_value(scope: ConfigScope, document: &Value) -> Result<Value> {
    let unlocated = |segments: &[String], kind: ConfigProblemKind, message: String| ConfigProblem {
        file: None,
        pointer: pointer_of(segments),
        line: None,
        column: None,
        message,
        kind,
    };
    project(scope, document, &unlocated).map(|(effective, _)| effective).map_err(|problems| ClaudineError::ConfigInvalid { problems })
}

/// Builds the problem at `segments` (for [`ConfigProblemKind::UnknownKey`],
/// the path of the key itself).
type Problem<'a> = dyn Fn(&[String], ConfigProblemKind, String) -> ConfigProblem + 'a;

/// The effective document and its defaults, or every problem in it.
fn project(
    scope: ConfigScope,
    document: &Value,
    problem: &Problem<'_>,
) -> std::result::Result<(Value, Vec<AppliedDefault>), Vec<ConfigProblem>> {
    let Value::Object(map) = document else {
        let what = if document.is_null() { "is empty" } else { "is not a mapping" };
        return Err(vec![problem(
            &[],
            ConfigProblemKind::Syntax,
            format!("the configuration {what}; write `{{}}` for one that keeps every default"),
        )]);
    };
    let flat = super::wire::flat_entries(document);
    // The document-level rules below are judged whatever the schema or the
    // entries' form says, so one diagnosis never hides another: correcting
    // the reported problems must not reveal a further one.
    let (defaults, mut problems) = if flat.is_empty() {
        match contract(scope).validate_strict(document) {
            Ok(success) => (Some(success.defaults), Vec::new()),
            Err(failure) => (None, failure_problems(&failure, problem)),
        }
    } else {
        // A flat action or route fails every keyed arm, so the document is
        // judged in its keyed view instead: the migration message is the
        // entry's one diagnosis of its form, and whatever else is wrong in
        // it (a misspelled setting, a wrong type) is still reported, at the
        // key the file wrote.
        let mut keyed = document.clone();
        super::wire::upgrade_flat(&mut keyed);
        let authored = |segments: &[String], kind: ConfigProblemKind, message: String| {
            let segments = flat.iter().find_map(|entry| entry.authored(segments)).unwrap_or_else(|| segments.to_vec());
            problem(&segments, kind, message)
        };
        let mut problems = match contract(scope).validate_strict(&keyed) {
            Ok(_) => Vec::new(),
            Err(failure) => failure_problems(&failure, &authored),
        };
        problems.extend(flat.into_iter().map(|entry| problem(&entry.segments, ConfigProblemKind::InvalidSetting, entry.message)));
        (None, problems)
    };
    if map.contains_key("preferred_agent") && map.contains_key("favorite_agent") {
        problems.push(problem(
            &["favorite_agent".to_string()],
            ConfigProblemKind::InvalidSetting,
            "`favorite_agent` is the former name of `preferred_agent`; write only one of them".into(),
        ));
    }
    exit_expression_problems(map.get("exit_expressions"), problem, &mut problems);
    let Some(defaults) = defaults.filter(|_| problems.is_empty()) else {
        return Err(problems);
    };

    let mut effective = document.clone();
    for default in &defaults {
        insert(&mut effective, default);
    }
    // Darkmatter owns `permissions` and reads its defaults itself.
    if let Value::Object(effective) = &mut effective {
        match map.get("permissions") {
            Some(permissions) => {
                effective.insert("permissions".into(), permissions.clone());
            }
            None => {
                effective.remove("permissions");
            }
        }
    }
    Ok((effective, defaults))
}

/// An exit-expression entry writes exactly one of `pattern` and `patterns`,
/// which the schema cannot say.
fn exit_expression_problems(value: Option<&Value>, problem: &Problem<'_>, out: &mut Vec<ConfigProblem>) {
    let (rules, prefix): (&[Value], Vec<String>) = match value {
        Some(Value::Array(rules)) => (rules, vec!["exit_expressions".into()]),
        Some(Value::Object(layer)) => match layer.get("rules") {
            Some(Value::Array(rules)) => (rules, vec!["exit_expressions".into(), "rules".into()]),
            _ => return,
        },
        _ => return,
    };
    for (index, rule) in rules.iter().enumerate() {
        let written = |key: &str| rule.get(key).is_some_and(|value| !value.is_null());
        let message = match (written("pattern"), written("patterns")) {
            (true, true) => "write `pattern` or `patterns`, not both",
            (false, false) => "write `pattern` or `patterns`",
            _ => continue,
        };
        let mut segments = prefix.clone();
        segments.push(index.to_string());
        out.push(problem(&segments, ConfigProblemKind::InvalidSetting, message.into()));
    }
}

/// Every problem of a failed validation, located.
fn failure_problems(failure: &ValidationFailure, problem: &Problem<'_>) -> Vec<ConfigProblem> {
    match failure {
        ValidationFailure::InvalidData { problems } | ValidationFailure::UnavailableCheck { problems } => {
            let mut located: Vec<ConfigProblem> = Vec::new();
            for found in problems {
                let segments = found.instance_path.segments().to_vec();
                let unknown: Vec<&String> = if found.offending_properties.is_empty() {
                    found.offending_property.iter().collect()
                } else {
                    found.offending_properties.iter().collect()
                };
                let is_unknown = found.code == ValidationProblemCode::UnknownKey
                    || found.message.contains("Additional properties");
                if is_unknown && !unknown.is_empty() {
                    for key in unknown {
                        let mut at = segments.clone();
                        at.push(key.clone());
                        located.push(problem(&at, ConfigProblemKind::UnknownKey, format!("`{key}` is not a setting here")));
                    }
                } else {
                    located.push(problem(&segments, ConfigProblemKind::InvalidSetting, describe(found)));
                }
            }
            let mut seen = std::collections::HashSet::new();
            located.retain(|found| seen.insert((found.kind, found.pointer.clone(), found.message.clone())));
            located
        }
        ValidationFailure::Unprepared { error } => vec![problem(
            &[],
            ConfigProblemKind::InvalidSetting,
            format!("the configuration schema cannot be prepared: {error}"),
        )],
        other => vec![problem(&[], ConfigProblemKind::InvalidSetting, format!("{other:?}"))],
    }
}

/// A validation message worded for a configuration reader.
fn describe(problem: &ValidationProblem) -> String {
    if problem.message.contains("is not valid under any of the schemas") {
        "the value matches none of the forms this setting accepts".to_string()
    } else {
        problem.message.clone()
    }
}

/// Inserts one default at its path, parents first.
fn insert(document: &mut Value, default: &AppliedDefault) {
    let Some((last, parents)) = default.path.segments().split_last() else {
        return;
    };
    let mut target = &mut *document;
    for segment in parents {
        target = match target {
            Value::Object(map) => match map.get_mut(segment.as_str()) {
                Some(child) => child,
                None => return,
            },
            Value::Array(items) => match segment.parse::<usize>().ok().and_then(|index| items.get_mut(index)) {
                Some(child) => child,
                None => return,
            },
            _ => return,
        };
    }
    if let Value::Object(map) = target {
        map.insert(last.clone(), default.value.clone());
    }
}

/// Locates problems in one loaded file.
struct Locator<'a> {
    path: &'a Path,
    text: &'a str,
    loaded: &'a LoadedYaml,
}

impl Locator<'_> {
    fn problem(&self, segments: &[String], kind: ConfigProblemKind, message: String) -> ConfigProblem {
        let located = match (kind, segments.split_last()) {
            (ConfigProblemKind::UnknownKey, Some((key, parent))) => self.key_range(parent, key),
            _ => None,
        };
        let at = located.or_else(|| self.range(segments)).map(|range| line_column(self.text, range.start));
        ConfigProblem {
            file: Some(self.path.to_path_buf()),
            pointer: pointer_of(segments),
            line: Some(at.map_or(1, |(line, _)| line)),
            column: Some(at.map_or(1, |(_, column)| column)),
            message,
            kind,
        }
    }

    /// The text of the key `key` in the mapping at `parent`.
    fn key_range(&self, parent: &[String], key: &str) -> Option<Range<usize>> {
        let mut problem = locating_problem(parent);
        problem.code = ValidationProblemCode::UnknownKey;
        problem.offending_property = Some(key.to_string());
        self.loaded.byte_range(&problem)
    }

    /// The text of the value at `segments`, or of its nearest located parent.
    fn range(&self, segments: &[String]) -> Option<Range<usize>> {
        (0..=segments.len()).rev().find_map(|length| {
            if length == 0 {
                return None;
            }
            self.loaded.byte_range(&locating_problem(&segments[..length]))
        })
    }
}

fn pointer_of(segments: &[String]) -> String {
    simplified_schema::yaml_documents::pointer_string(segments)
}

/// The 1-based line and character column of byte `offset` in `text`.
pub(crate) fn line_column(text: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(text.len());
    let before = &text[..text.floor_char_boundary(offset)];
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |at| at + 1);
    let column = before[line_start..].trim_start_matches('\u{feff}').chars().count() + 1;
    (line, column)
}
